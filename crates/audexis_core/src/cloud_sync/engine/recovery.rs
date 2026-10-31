use super::*;

impl CloudSyncContext {
    pub async fn retry(&self, operation: &str, keep_local: bool) -> Result<()> {
        let payload: String = sqlx::query_scalar(
            "SELECT payload FROM cloud_outbox WHERE connection_id=? AND operation_id=?",
        )
        .bind(&self.connection_id)
        .bind(operation)
        .fetch_one(&self.db.pool)
        .await?;
        let mut job: Job = serde_json::from_str(&payload)?;
        if !keep_local {
            let mut tx = self.db.pool.begin_with("BEGIN IMMEDIATE").await?;
            let mut restore_playlist = None;
            if let Job::Mutate {
                playlist_id: Some(id),
                request:
                    Mutation {
                        edit: Edit::Playlist { playlist_id, .. },
                        ..
                    },
                ..
            } = &job
            {
                let doc: Option<String> = sqlx::query_scalar(
                    "SELECT document FROM cloud_playlists WHERE connection_id=? AND id=?",
                )
                .bind(&self.connection_id)
                .bind(playlist_id)
                .fetch_optional(&mut *tx)
                .await?;
                if let Some(doc) = doc {
                    let remote: Playlist = serde_json::from_str(&doc)?;
                    let name: Option<String> =
                        sqlx::query_scalar("SELECT name FROM playlists WHERE id=?")
                            .bind(id)
                            .fetch_optional(&mut *tx)
                            .await?;
                    let ids:Vec<Option<String>>=sqlx::query_scalar("SELECT l.cloud_track_id FROM playlist_tracks pt LEFT JOIN file_cloud_links l ON l.file_id=pt.file_id AND l.connection_id=? WHERE pt.playlist_id=? ORDER BY pt.ord").bind(&self.connection_id).bind(id).fetch_all(&mut *tx).await?;
                    sqlx::query("UPDATE cloud_playlist_links SET snapshot=? WHERE connection_id=? AND playlist_id=?").bind(serde_json::to_string(&(name.unwrap_or_default(),ids.into_iter().flatten().collect::<Vec<_>>()))?).bind(&self.connection_id).bind(id).execute(&mut *tx).await?;
                    restore_playlist = Some(Change {
                        cursor: 0,
                        value: ChangeValue::Playlist(remote),
                    });
                }
            }
            if let Job::Mutate {
                file_id: Some(file),
                request:
                    Mutation {
                        edit: Edit::Track { track_id, .. },
                        ..
                    },
                ..
            } = &job
            {
                let doc: String = sqlx::query_scalar(
                    "SELECT document FROM cloud_tracks WHERE connection_id=? AND id=?",
                )
                .bind(&self.connection_id)
                .bind(track_id)
                .fetch_one(&mut *tx)
                .await?;
                let track: Track = serde_json::from_str(&doc)?;
                store::write_metadata(&mut tx, *file, &track.metadata).await?;
                sqlx::query(
                    "UPDATE cloud_local_state SET metadata=?,revision=? WHERE connection_id=? AND file_id=?",
                )
                .bind(serde_json::to_string(&track.metadata)?)
                .bind(track.revision)
                .bind(&self.connection_id)
                .bind(file)
                .execute(&mut *tx)
                .await?;
            }
            sqlx::query("DELETE FROM cloud_outbox WHERE connection_id=? AND operation_id=?")
                .bind(&self.connection_id)
                .bind(operation)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            if let Some(change) = restore_playlist {
                store::apply(&self.db.pool, &self.connection_id, &[change], None).await?;
            }
            self.files.publish_changed(Vec::new());
            return Ok(());
        }
        let new_id = uuid::Uuid::new_v4().to_string();
        match &mut job {
            Job::Upload {
                file_id,
                request,
                upload_id,
                uploaded,
            } => {
                let local:Option<(String,String,i64)>=sqlx::query_as("SELECT hash,file_name,size FROM files WHERE id=? AND hash IS NOT NULL AND missing_since IS NULL").bind(*file_id).fetch_optional(&self.db.pool).await?;
                if let Some((hash, name, size)) = local {
                    request.sha256 = hash;
                    request.file_name = name;
                    request.size = size;
                    let mut conn = self.db.pool.acquire().await?;
                    request.metadata = store::local_metadata(&mut conn, *file_id).await?;
                }
                if let Some(id) = upload_id.as_ref() {
                    let _ = self.client()?.cancel_upload(id).await;
                }
                request.operation_id = new_id.clone();
                *upload_id = None;
                *uploaded = false;
            }
            Job::Mutate { request, .. } => {
                request.operation_id = new_id.clone();
                let (id, table) = match &request.edit {
                    Edit::Track { track_id, .. } | Edit::DeleteTrack { track_id } => {
                        (track_id, "cloud_tracks")
                    }
                    Edit::Playlist { playlist_id, .. } | Edit::DeletePlaylist { playlist_id } => {
                        (playlist_id, "cloud_playlists")
                    }
                };
                let sql = format!("SELECT revision FROM {table} WHERE connection_id=? AND id=?");
                request.base_revision = sqlx::query_scalar(&sql)
                    .bind(&self.connection_id)
                    .bind(id)
                    .fetch_optional(&self.db.pool)
                    .await?
                    .unwrap_or(0);
            }
        }
        sqlx::query("UPDATE cloud_outbox SET operation_id=?,payload=?,state='pending',attempts=0,retry_at=0,error=NULL WHERE connection_id=? AND operation_id=?").bind(new_id).bind(serde_json::to_string(&job)?).bind(&self.connection_id).bind(operation).execute(&self.db.pool).await?;
        Ok(())
    }
}
