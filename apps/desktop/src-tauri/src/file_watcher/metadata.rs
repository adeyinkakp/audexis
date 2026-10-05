use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::UNIX_EPOCH;

use symphonia::core::formats::{probe::Hint, TrackType};
use symphonia::core::io::MediaSourceStream;
use tauri::{async_runtime, AppHandle, Manager};

use crate::database::Database;
use crate::tag_manager::tag_backend::{DefaultBackend, TagBackend};
use crate::tag_manager::utils::{MetadataFile, TagValue};
use crate::utils::errors::DatabaseError;
use crate::AppState;

use super::{events, FileWatcher};

struct PendingWorkerGuard(Arc<AtomicBool>);

impl Drop for PendingWorkerGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl FileWatcher {
    pub(super) async fn handle_pending(app_handle: &AppHandle) -> Result<(), DatabaseError> {
        let state: tauri::State<'_, AppState> = app_handle.state::<AppState>();
        let db: &Database = &state.db;
        if state
            .pending_worker_running
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Ok(());
        }
        let _worker_guard = PendingWorkerGuard(state.pending_worker_running.clone());
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM files WHERE status = 'pending' AND metadata_status = 'ok' AND missing_since IS NULL",
        )
        .fetch_one(&db.pool)
        .await
        .map_err(DatabaseError::Sqlx)? as u64;
        let task = crate::utils::tasks::TaskGuard::start(
            app_handle,
            "metadata-index",
            "Indexing music metadata",
            Some(total),
        );
        let mut processed = 0_u64;

        loop {
            let pending: Vec<(i64, String)> = sqlx::query_as(
                "SELECT id, path
                 FROM files
                 WHERE status = 'pending' AND metadata_status = 'ok' AND missing_since IS NULL
                 ORDER BY id
                 LIMIT 50",
            )
            .fetch_all(&db.pool)
            .await
            .map_err(DatabaseError::Sqlx)?;

            if pending.is_empty() {
                task.complete(format!("Indexed {processed} files"));
                return Ok(());
            }
            let batch_size = pending.len() as u64;

            let parsed = async_runtime::spawn_blocking(move || {
                let backend = DefaultBackend::new();
                pending
                    .into_iter()
                    .map(|(id, path)| {
                        let result = backend.read(&PathBuf::from(&path));
                        (id, path, result)
                    })
                    .collect::<Vec<_>>()
            })
            .await
            .map_err(DatabaseError::Tauri)?;

            for (id, _path, result) in parsed {
                let metadata = match result {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        sqlx::query(
                            "UPDATE files
                               SET metadata_status = 'failed' WHERE id = ?1",
                        )
                        .bind(id)
                        .execute(&db.pool)
                        .await
                        .map_err(DatabaseError::Sqlx)?;
                        drop(error);
                        continue;
                    }
                };

                Self::store_metadata(&db.pool, id, &metadata).await?;
                events::emit_changed(app_handle, &[id])?;
            }
            processed += batch_size;
            task.update(
                processed,
                Some(total),
                Some(format!("{processed} of {total} files")),
            );
        }
    }

    pub(crate) async fn store_metadata(
        pool: &sqlx::SqlitePool,
        file_id: i64,
        metadata: &MetadataFile,
    ) -> Result<(), DatabaseError> {
        let path = metadata.path.clone();
        let duration_ms = async_runtime::spawn_blocking(move || read_duration_ms(&path))
            .await
            .map_err(DatabaseError::Tauri)?;
        let mut tx = pool.begin().await.map_err(DatabaseError::Sqlx)?;

        sqlx::query("DELETE FROM metadata_texts WHERE file_id = ?1")
            .bind(file_id)
            .execute(&mut *tx)
            .await
            .map_err(DatabaseError::Sqlx)?;
        sqlx::query("DELETE FROM metadata_pictures WHERE file_id = ?1")
            .bind(file_id)
            .execute(&mut *tx)
            .await
            .map_err(DatabaseError::Sqlx)?;

        for (key, values) in &metadata.tags {
            for (ord, value) in values.iter().enumerate() {
                match value {
                    TagValue::Picture {
                        mime,
                        data,
                        picture_type,
                        description,
                    } => {
                        sqlx::query(
                            "INSERT INTO metadata_pictures
                             (file_id, data, mime_type, picture_type, description)
                             VALUES (?1, ?2, ?3, ?4, ?5)",
                        )
                        .bind(file_id)
                        .bind(data)
                        .bind(mime)
                        .bind(i64::from(picture_type.unwrap_or(3)))
                        .bind(description.as_deref().unwrap_or_default())
                        .execute(&mut *tx)
                        .await
                        .map_err(DatabaseError::Sqlx)?;
                    }
                    _ => {
                        sqlx::query(
                            "INSERT INTO metadata_texts (file_id, key, value, ord)
                             VALUES (?1, ?2, ?3, ?4)",
                        )
                        .bind(file_id)
                        .bind(key.to_string())
                        .bind(value.to_string())
                        .bind(ord as i64)
                        .execute(&mut *tx)
                        .await
                        .map_err(DatabaseError::Sqlx)?;
                    }
                }
            }
        }

        let validated_at = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or_default();

        sqlx::query(
            "UPDATE files
               SET status = 'ok', format = ?1, last_validated = ?2, duration_ms = ?3
               WHERE id = ?4",
        )
        .bind(metadata.tag_format.to_string())
        .bind(validated_at)
        .bind(duration_ms)
        .bind(file_id)
        .execute(&mut *tx)
        .await
        .map_err(DatabaseError::Sqlx)?;

        tx.commit().await.map_err(DatabaseError::Sqlx)
    }
}

pub(super) fn read_duration_ms(path: &Path) -> Option<i64> {
    let source = crate::utils::library_files::open(path).ok()?;
    let stream = MediaSourceStream::new(Box::new(source), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        hint.with_extension(extension);
    }
    let mut probed = symphonia::default::get_probe()
        .probe(&hint, stream, Default::default(), Default::default())
        .ok()?;
    let track_id = probed.default_track(TrackType::Audio)?.id;
    let duration = crate::audio_player::duration::resolve_duration_ms(&mut probed, track_id, None);
    (duration.ms > 0)
        .then(|| i64::try_from(duration.ms).ok())
        .flatten()
}
