use crate::tag_manager::tag_backend::{DefaultBackend, TagBackend};
use crate::utils::errors::DatabaseError;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use tauri::{async_runtime, AppHandle};

pub(super) async fn backfill(pool: &SqlitePool, app: &AppHandle) -> Result<(), DatabaseError> {
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM files WHERE duration_ms IS NULL OR duration_ms <= 0 OR format IS NULL OR format = '' OR last_validated = 0 OR size = 0 OR modified_at = 0",
    )
    .fetch_one(pool)
    .await
    .map_err(DatabaseError::Sqlx)? as u64;
    let task = crate::utils::tasks::TaskGuard::start(
        app,
        "metadata-maintenance",
        "Updating library metadata",
        Some(total),
    );
    let mut processed = 0_u64;
    let mut cursor: i64 = 0;
    loop {
        let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
            "SELECT id, path, format FROM files WHERE id > ?1 AND
             (duration_ms IS NULL OR duration_ms <= 0 OR format IS NULL OR format = ''
              OR last_validated = 0 OR size = 0 OR modified_at = 0)
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
        let changed_ids: Vec<_> = rows.iter().map(|row| row.0).collect();
        let values = async_runtime::spawn_blocking(move || {
            rows.into_iter()
                .filter_map(|(id, path, format)| {
                    let path = PathBuf::from(path);
                    let stat = std::fs::metadata(&path).ok()?;
                    let duration = super::read_duration_ms(&path);
                    let format = format.filter(|value| !value.is_empty()).or_else(|| {
                        DefaultBackend::new()
                            .read(&path)
                            .ok()
                            .map(|metadata| metadata.tag_format.to_string())
                    });
                    let modified = stat
                        .modified()
                        .ok()?
                        .duration_since(UNIX_EPOCH)
                        .ok()?
                        .as_secs() as i64;
                    Some((id, duration, format, stat.len() as i64, modified))
                })
                .collect::<Vec<_>>()
        })
        .await
        .map_err(DatabaseError::Tauri)?;
        for (id, duration, format, size, modified) in values {
            sqlx::query(
                "UPDATE files SET
                 duration_ms = CASE WHEN duration_ms IS NULL OR duration_ms <= 0 THEN ?1 ELSE duration_ms END,
                 format = COALESCE(NULLIF(format, ''), ?2),
                 size = CASE WHEN size = 0 THEN ?3 ELSE size END,
                 modified_at = CASE WHEN modified_at = 0 THEN ?4 ELSE modified_at END,
                 last_validated = CASE WHEN last_validated = 0 AND ?1 IS NOT NULL THEN unixepoch() ELSE last_validated END
                 WHERE id = ?5",
            ).bind(duration).bind(format).bind(size).bind(modified).bind(id)
                .execute(pool).await.map_err(DatabaseError::Sqlx)?;
        }
        super::events::emit_changed(app, &changed_ids)?;
        processed += changed_ids.len() as u64;
        task.update(
            processed,
            Some(total),
            Some(format!("{processed} of {total} files")),
        );
    }
    task.complete(format!("Updated {processed} files"));
    Ok(())
}
