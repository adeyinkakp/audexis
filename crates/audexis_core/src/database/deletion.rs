use super::Database;
use crate::{
    cloud_sync::{
        CloudSyncContext,
        protocol::{Change, Edit, Mutation, MutationResult, Track},
        store,
    },
    library_files::LibraryFiles,
};

impl Database {
    pub async fn delete_library_song(
        &self,
        files: &LibraryFiles,
        cloud: Option<&CloudSyncContext>,
        id: i64,
    ) -> Result<(), String> {
        self.delete_library_song_inner(files, cloud, id)
            .await
            .map_err(|error| error.to_string())
    }

    async fn delete_library_song_inner(
        &self,
        files: &LibraryFiles,
        cloud: Option<&CloudSyncContext>,
        id: i64,
    ) -> Result<(), crate::cloud_sync::SyncError> {
        use crate::cloud_sync::SyncError;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let path: Option<String> = sqlx::query_scalar("SELECT path FROM files WHERE id=?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        let Some(path) = path else { return Ok(()) };
        let links: Vec<(String, String)> = sqlx::query_as(
            "SELECT connection_id,cloud_track_id FROM file_cloud_links WHERE file_id=?",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        if links
            .iter()
            .any(|(connection, _)| cloud.is_none_or(|c| c.connection_id != *connection))
        {
            return Err(SyncError::local(
                "This song is linked to another cloud account. Connect that account before deleting it.",
            ));
        }
        let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cloud_outbox WHERE json_extract(payload,'$.file_id')=? OR json_extract(payload,'$.request.edit.track_id') IN (SELECT cloud_track_id FROM file_cloud_links WHERE file_id=?))")
            .bind(id).bind(id).fetch_one(&mut *tx).await?;
        if pending {
            return Err(SyncError::local(
                "This song has pending cloud changes. Finish or resolve them in Cloud Sync before deleting it.",
            ));
        }
        if path.starts_with("cloud-library://") && links.is_empty() {
            return Err(SyncError::local(
                "The cloud link for this song is unavailable. Sync your library and try again.",
            ));
        }
        let mut remote = Vec::new();
        for (connection, track_id) in &links {
            let document: String = sqlx::query_scalar(
                "SELECT document FROM cloud_tracks WHERE connection_id=? AND id=?",
            )
            .bind(connection)
            .bind(track_id)
            .fetch_one(&mut *tx)
            .await?;
            let track: Track = serde_json::from_str(&document)?;
            remote.push(track);
        }
        if let Some(context) = cloud {
            let client = context.client()?;
            if !remote.is_empty() {
                client.verify(&context.server_user_id).await?;
            }
            for track in &remote {
                if !track.deleted {
                    let result: MutationResult = client
                        .post(
                            "/sync/mutations",
                            &Mutation {
                                operation_id: uuid::Uuid::new_v4().to_string(),
                                base_revision: track.revision,
                                edit: Edit::DeleteTrack {
                                    track_id: track.id.clone(),
                                },
                            },
                        )
                        .await?;
                    store::apply_to_connection(
                        &mut tx,
                        &context.connection_id,
                        &[Change {
                            cursor: 0,
                            value: result.value,
                        }],
                        None,
                    )
                    .await?;
                }
            }
        }
        tx.commit().await?;
        files.reconcile(Default::default(), vec![id]);
        if let Some(context) = cloud {
            for track in &remote {
                context.remove_download(track).await?;
            }
        }
        if !crate::library_files::is_virtual_path(std::path::Path::new(&path)) {
            match tokio::fs::remove_file(&path).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(SyncError::local(format!(
                        "Could not delete {path}: {error}. Any completed cloud deletion cannot be undone; the local library record was kept so you can retry."
                    )));
                }
            }
        }
        sqlx::query("DELETE FROM files WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        files.publish_changed(vec![id]);
        Ok(())
    }
}
