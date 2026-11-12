use super::WatcherContext;
use super::WatcherError as DatabaseError;
use crate::tag_manager::tag_backend::DefaultBackend;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use tokio::task as async_runtime;

pub(super) async fn backfill(pool: &SqlitePool, app: &WatcherContext) -> Result<(), DatabaseError> {
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM files WHERE cloud_connection_id IS NULL AND missing_since IS NULL AND
         (duration_ms IS NULL OR duration_ms <= 0 OR format IS NULL OR format = '' OR last_validated = 0 OR size = 0 OR modified_at = 0 OR hash IS NULL OR hash = '')",
    )
    .fetch_one(pool)
    .await
    .map_err(DatabaseError::Sqlx)? as u64;
    let task = super::events::TaskGuard::start(
        app,
        "metadata-maintenance",
        "Updating library metadata",
        Some(total),
    );
    let mut processed = 0_u64;
    let mut cursor: i64 = 0;
    loop {
        let rows: Vec<(i64, String, Option<String>, Option<i64>, Option<String>)> = sqlx::query_as(
            "SELECT id, path, format, duration_ms, hash FROM files WHERE id > ?1 AND cloud_connection_id IS NULL AND missing_since IS NULL AND
             (duration_ms IS NULL OR duration_ms <= 0 OR format IS NULL OR format = ''
              OR last_validated = 0 OR size = 0 OR modified_at = 0 OR hash IS NULL OR hash = '')
             ORDER BY id LIMIT 50",
        )
        .bind(cursor)
        .fetch_all(pool)
        .await
        .map_err(DatabaseError::Sqlx)?;
        if rows.is_empty() {
            break;
        }
        cursor = rows.last().unwrap().0;
        for (id, path, format, duration, hash) in rows {
            let value = async_runtime::spawn_blocking(move || {
                let path = PathBuf::from(path);
                let stat = crate::library_files::metadata(&path).ok()?;
                let duration = duration
                    .filter(|duration| *duration > 0)
                    .or_else(|| super::metadata::read_duration_ms(&path));
                let hash = hash
                    .filter(|hash| !hash.is_empty())
                    .or_else(|| super::hashing::attempt_file_hash(&path));
                let format = format.filter(|value| !value.is_empty()).or_else(|| {
                    let format = DefaultBackend::new().resolve_format(&path);
                    (format != crate::tag_manager::traits::Formats::Unknown)
                        .then(|| format.to_string())
                });
                let modified = stat
                    .modified()
                    .ok()?
                    .duration_since(UNIX_EPOCH)
                    .ok()?
                    .as_secs() as i64;
                Some((duration, format, stat.len() as i64, modified, hash))
            })
            .await
            .map_err(DatabaseError::Join)?;
            processed += 1;
            if let Some((duration, format, size, modified, hash)) = value {
                sqlx::query(
                "UPDATE files SET
                 duration_ms = CASE WHEN duration_ms IS NULL OR duration_ms <= 0 THEN ?1 ELSE duration_ms END,
                 format = COALESCE(NULLIF(format, ''), ?2),
                 size = CASE WHEN size = 0 THEN ?3 ELSE size END,
                 modified_at = CASE WHEN modified_at = 0 THEN ?4 ELSE modified_at END,
                 hash = ?6,
                 last_validated = CASE WHEN last_validated = 0 AND ?1 IS NOT NULL THEN unixepoch() ELSE last_validated END
                 WHERE id = ?5",
            ).bind(duration).bind(format).bind(size).bind(modified).bind(id).bind(hash)
                .execute(pool).await.map_err(DatabaseError::Sqlx)?;
                super::events::emit_changed(app, &[id])?;
            }
            task.update(
                processed,
                Some(total),
                Some(format!("{processed} of {total} files")),
            );
        }
    }
    task.complete(format!("Updated {processed} files"));
    Ok(())
}
