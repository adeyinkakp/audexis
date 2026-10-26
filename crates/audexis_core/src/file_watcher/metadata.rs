use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::UNIX_EPOCH;

use super::WatcherContext;
use symphonia::core::formats::{TrackType, probe::Hint};
use symphonia::core::io::MediaSourceStream;
use tokio::task as async_runtime;

use super::WatcherError as DatabaseError;
use crate::database::Database;
use crate::tag_manager::tag_backend::{DefaultBackend, TagBackend};
use crate::tag_manager::utils::{MetadataFile, TagValue};

use super::{FileWatcher, events};

struct PendingWorkerGuard(Arc<AtomicBool>);

impl Drop for PendingWorkerGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl FileWatcher {
    pub(super) async fn handle_pending(app_handle: &WatcherContext) -> Result<(), DatabaseError> {
        let state = app_handle;
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
            "SELECT COUNT(*) FROM files WHERE cloud_connection_id IS NULL AND status = 'pending' AND metadata_status = 'ok' AND missing_since IS NULL",
        )
        .fetch_one(&db.pool)
        .await
        .map_err(DatabaseError::Sqlx)? as u64;
        let task = super::events::TaskGuard::start(
            app_handle,
            "metadata-index",
            "Indexing music metadata",
            Some(total),
        );
        let mut processed = 0_u64;
        let mut last_refresh = std::time::Instant::now();

        loop {
            let pending: Vec<(i64, String)> = sqlx::query_as(
                "SELECT id, path
                 FROM files
                 WHERE cloud_connection_id IS NULL AND status = 'pending' AND metadata_status = 'ok' AND missing_since IS NULL
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
            let mut changed = Vec::new();
            for (id, path) in pending {
                let read_path = PathBuf::from(&path);
                let result =
                    async_runtime::spawn_blocking(move || DefaultBackend::new().read(&read_path))
                        .await
                        .map_err(DatabaseError::Join)?;
                let metadata = match result {
                    Ok(metadata) => Some(metadata),
                    Err(error) => {
                        let hash = async_runtime::spawn_blocking(move || {
                            super::hashing::attempt_file_hash(Path::new(&path))
                        })
                        .await
                        .map_err(DatabaseError::Join)?;
                        sqlx::query(
                            "UPDATE files
                               SET metadata_status = 'failed', hash = ?2 WHERE id = ?1",
                        )
                        .bind(id)
                        .bind(hash)
                        .execute(&db.pool)
                        .await
                        .map_err(DatabaseError::Sqlx)?;
                        drop(error);
                        None
                    }
                };

                if let Some(metadata) = metadata {
                    Self::store_metadata(&db.pool, id, &metadata).await?;
                }
                changed.push(id);
                processed += 1;
                task.update(
                    processed,
                    Some(total),
                    Some(format!("{processed} of {total} files")),
                );
                if last_refresh.elapsed() >= std::time::Duration::from_millis(250) {
                    events::emit_changed(app_handle, &changed)?;
                    changed.clear();
                    last_refresh = std::time::Instant::now();
                }
            }
            events::emit_changed(app_handle, &changed)?;
        }
    }

    pub(crate) async fn store_metadata(
        pool: &sqlx::SqlitePool,
        file_id: i64,
        metadata: &MetadataFile,
    ) -> Result<(), DatabaseError> {
        let path = metadata.path.clone();
        let (duration_ms, hash) = async_runtime::spawn_blocking(move || {
            (
                read_duration_ms(&path),
                super::hashing::attempt_file_hash(&path),
            )
        })
        .await
        .map_err(DatabaseError::Join)?;
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
                        let (storage_key, storage_value) = match value {
                            TagValue::UserText(entry) => (
                                format!("custom:text:{}", entry.description),
                                entry.value.clone(),
                            ),
                            TagValue::UserUrl(entry) => (
                                format!("custom:url:{}", entry.description),
                                entry.url.clone(),
                            ),
                            TagValue::Comment { text, .. } => (key.to_string(), text.clone()),
                            _ => (key.to_string(), value.to_string()),
                        };
                        sqlx::query(
                            "INSERT INTO metadata_texts (file_id, key, value, ord)
                             VALUES (?1, ?2, ?3, ?4)",
                        )
                        .bind(file_id)
                        .bind(storage_key)
                        .bind(storage_value)
                        .bind(ord as i64)
                        .execute(&mut *tx)
                        .await
                        .map_err(DatabaseError::Sqlx)?;
                    }
                }
                if matches!(
                    value,
                    TagValue::Picture { .. } | TagValue::UserText(_) | TagValue::UserUrl(_)
                ) {
                    sqlx::query("INSERT INTO metadata_texts (file_id, key, value, ord) VALUES (?1, ?2, ?3, ?4)")
                        .bind(file_id).bind(key.to_string()).bind(value.to_string()).bind(ord as i64)
                        .execute(&mut *tx).await.map_err(DatabaseError::Sqlx)?;
                }
            }
        }

        let validated_at = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or_default();

        sqlx::query(
            "UPDATE files
               SET status = 'ok', format = ?1, last_validated = ?2, duration_ms = ?3, hash = ?5
               WHERE id = ?4",
        )
        .bind(metadata.tag_format.to_string())
        .bind(validated_at)
        .bind(duration_ms)
        .bind(file_id)
        .bind(hash)
        .execute(&mut *tx)
        .await
        .map_err(DatabaseError::Sqlx)?;

        tx.commit().await.map_err(DatabaseError::Sqlx)
    }
}

pub(super) fn read_duration_ms(path: &Path) -> Option<i64> {
    let source = crate::library_files::open(path).ok()?;
    use std::io::{Read, Seek, SeekFrom};
    let mut source = source;
    let mut header = [0; 8];
    if source.read_exact(&mut header).is_ok() && &header[..4] == b"fLaC" {
        if header[4] & 0x7f == 0 && header[5..8] == [0, 0, 34] {
            let mut info = [0; 34];
            if source.read_exact(&mut info).is_ok() {
                let packed = u64::from_be_bytes(info[10..18].try_into().ok()?);
                let rate = packed >> 44;
                let samples = packed & ((1u64 << 36) - 1);
                if rate > 0 && samples > 0 {
                    return Some((samples * 1000 / rate) as i64).filter(|ms| *ms > 0);
                }
            }
        }
    }
    source.seek(SeekFrom::Start(0)).ok()?;
    let stream = MediaSourceStream::new(Box::new(source), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        hint.with_extension(extension);
    }
    let mut probed = symphonia::default::get_probe()
        .probe(&hint, stream, Default::default(), Default::default())
        .ok()?;
    let track = probed.default_track(TrackType::Audio)?.clone();
    let tb = track.time_base?;
    let ticks = if let Some(duration) = track.duration {
        duration.get()
    } else {
        let mut end = 0;
        while let Ok(Some(packet)) = probed.next_packet() {
            if packet.track_id == track.id && packet.pts.get() >= 0 {
                end = end.max((packet.pts.get() as u64).saturating_add(packet.dur.get()));
            }
        }
        end
    };
    let ms = u128::from(ticks) * u128::from(tb.numer.get()) * 1000 / u128::from(tb.denom.get());
    i64::try_from(ms).ok().filter(|ms| *ms > 0)
}
