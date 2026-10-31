use super::*;

pub(super) async fn thumbnails(context: &CloudSyncContext, client: &CloudClient) -> Result<()> {
    let docs:Vec<String>=sqlx::query_scalar("SELECT document FROM cloud_tracks WHERE connection_id=? AND json_extract(document,'$.artwork')=1 AND json_extract(document,'$.deleted')=0 ORDER BY random() LIMIT 30").bind(&context.connection_id).fetch_all(&context.db.pool).await?;
    let mut attempted = 0;
    for doc in docs {
        let track: Track = serde_json::from_str(&doc)?;
        let path = context.artwork_path(&track);
        if !path.exists() {
            if attempted >= 3 {
                continue;
            }
            attempted += 1;
            if let Ok(media) = client.media(&track.id, "artwork").await {
                let _ = client.download(&media.url, &path, None, 20_000_000).await;
            }
        }
        let library_path = format!("cloud-library://{}/{}", context.connection_id, track.id);
        let needs: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM files WHERE cloud_connection_id=? AND path=? AND NOT EXISTS(SELECT 1 FROM metadata_pictures p WHERE p.file_id=files.id))")
            .bind(&context.connection_id).bind(&library_path).fetch_one(&context.db.pool).await?;
        if needs {
            if let Ok(bytes) = tokio::fs::read(&path).await {
                sqlx::query("INSERT INTO metadata_pictures(file_id,data,mime_type,picture_type) SELECT id,?,'image/png',3 FROM files WHERE cloud_connection_id=? AND path=? AND NOT EXISTS(SELECT 1 FROM metadata_pictures p WHERE p.file_id=files.id)")
                    .bind(bytes).bind(&context.connection_id).bind(library_path).execute(&context.db.pool).await?;
                let ids: Vec<i64> = sqlx::query_scalar(
                    "SELECT id FROM files WHERE cloud_connection_id=? AND path=?",
                )
                .bind(&context.connection_id)
                .bind(format!(
                    "cloud-library://{}/{}",
                    context.connection_id, track.id
                ))
                .fetch_all(&context.db.pool)
                .await?;
                context.files.publish_artwork_changed(ids);
            }
        }
    }
    Ok(())
}
pub(super) async fn pull(context: &CloudSyncContext, client: &CloudClient) -> Result<()> {
    let mut cursor: i64 =
        sqlx::query_scalar("SELECT cursor FROM cloud_sync_cursors WHERE connection_id=?")
            .bind(&context.connection_id)
            .fetch_optional(&context.db.pool)
            .await?
            .unwrap_or(0);
    loop {
        let page: Changes = client.get(&format!("/sync/changes?after={cursor}")).await?;
        if page.next_cursor < cursor
            || page.changes.len() > 200
            || page.changes.windows(2).any(|w| w[0].cursor >= w[1].cursor)
            || page
                .changes
                .iter()
                .any(|c| c.cursor <= cursor || c.cursor > page.next_cursor)
            || page.has_more && page.next_cursor == cursor
        {
            return Err(SyncError::local("Server returned an invalid sync cursor"));
        }
        store::apply(
            &context.db.pool,
            &context.connection_id,
            &page.changes,
            Some(page.next_cursor),
        )
        .await?;
        if !page.changes.is_empty() {
            context.files.publish_changed(Vec::new());
        }
        cursor = page.next_cursor;
        if !page.has_more {
            break;
        }
    }
    Ok(())
}
