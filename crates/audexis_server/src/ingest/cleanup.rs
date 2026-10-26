use super::*;

pub(super) async fn cleanup(state: &AppState) -> Result<()> {
    if let Some(root) = &state.ingest_dir {
        let active: Vec<String> = sqlx::query_scalar("SELECT stage_key FROM cloud_uploads WHERE (state IN ('created','uploaded','failed') AND expires_at>extract(epoch FROM now())::bigint) OR (state='processing' AND lease_at>extract(epoch FROM now())::bigint-600)").fetch_all(&state.db).await?;
        let active: std::collections::HashSet<_> = active.into_iter().collect();
        if let Ok(mut files) = tokio::fs::read_dir(root.join("staging")).await {
            while let Some(file) = files.next_entry().await? {
                let key = format!("staging/{}", file.file_name().to_string_lossy());
                let meta = file.metadata().await?;
                if meta.is_file()
                    && !active.contains(&key)
                    && meta.modified()?.elapsed().unwrap_or_default().as_secs() > 3600
                {
                    let _ = tokio::fs::remove_file(file.path()).await;
                }
            }
        }
    }
    let keys:Vec<String>=sqlx::query_scalar("UPDATE cloud_uploads SET state='expired' WHERE expires_at<extract(epoch FROM now())::bigint AND (state IN ('created','uploaded','failed') OR (state='processing' AND lease_at<extract(epoch FROM now())::bigint-600)) RETURNING stage_key").fetch_all(&state.db).await?;
    for key in keys {
        let _ = state.delete_stage(&key).await;
    }
    let abandoned: Vec<String> = sqlx::query_scalar("SELECT w.object_key FROM cloud_object_writes w WHERE w.created_at<extract(epoch FROM now())::bigint-3600 AND NOT EXISTS(SELECT 1 FROM cloud_objects o WHERE o.object_key=w.object_key OR o.artwork_key=w.object_key) AND NOT EXISTS(SELECT 1 FROM cloud_media_variants v WHERE v.object_key=w.object_key) AND NOT EXISTS(SELECT 1 FROM cloud_uploads u WHERE u.stage_key=w.object_key AND u.state='processing' AND u.lease_at>=extract(epoch FROM now())::bigint-600) LIMIT 50").fetch_all(&state.db).await?;
    for key in abandoned {
        state.storage.delete(&state.client, &key).await?;
        sqlx::query("DELETE FROM cloud_object_writes WHERE object_key=$1")
            .bind(key)
            .execute(&state.db)
            .await?;
    }
    sqlx::query("DELETE FROM cloud_object_writes w WHERE EXISTS(SELECT 1 FROM cloud_objects o WHERE o.object_key=w.object_key OR o.artwork_key=w.object_key)").execute(&state.db).await?;
    sqlx::query("UPDATE cloud_objects o SET unreferenced_at=extract(epoch FROM now())::bigint WHERE unreferenced_at IS NULL AND NOT EXISTS(SELECT 1 FROM cloud_tracks t WHERE t.object_hash=o.sha256 AND NOT t.deleted)").execute(&state.db).await?;
    let hashes:Vec<String>=sqlx::query_scalar("SELECT o.sha256 FROM cloud_objects o WHERE o.unreferenced_at<extract(epoch FROM now())::bigint-86400 AND NOT EXISTS(SELECT 1 FROM cloud_tracks t WHERE t.object_hash=o.sha256 AND NOT t.deleted) LIMIT 10").fetch_all(&state.db).await?;
    for hash in hashes {
        let mut tx = state.db.begin().await?;
        let lock = i64::from_be_bytes(Sha256::digest(hash.as_bytes())[..8].try_into().unwrap());
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(lock)
            .execute(&mut *tx)
            .await?;
        let row=sqlx::query("SELECT object_key,artwork_key FROM cloud_objects o WHERE sha256=$1 AND NOT EXISTS(SELECT 1 FROM cloud_tracks t WHERE t.object_hash=o.sha256 AND NOT t.deleted) FOR UPDATE").bind(&hash).fetch_optional(&mut *tx).await?;
        if let Some(row) = row {
            let variants: Vec<String> = sqlx::query_scalar("SELECT object_key FROM cloud_media_variants WHERE object_hash=$1 AND object_key IS NOT NULL").bind(&hash).fetch_all(&mut *tx).await?;
            for key in variants {
                state.storage.delete(&state.client, &key).await?;
            }
            let key: String = row.get("object_key");
            state.storage.delete(&state.client, &key).await?;
            if let Some(art) = row.get::<Option<String>, _>("artwork_key") {
                state.storage.delete(&state.client, &art).await?;
            }
            sqlx::query("DELETE FROM cloud_objects WHERE sha256=$1")
                .bind(&hash)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
    }
    Ok(())
}
