use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::WatcherContext;
use notify_debouncer_full::DebouncedEvent;
use sqlx::{QueryBuilder, Sqlite};
use tokio::task as async_runtime;
use walkdir::WalkDir;

use super::WatcherError as DatabaseError;
use super::get_library_roots;
use crate::database::Database;

use super::classification::{ClassifiedEvents, classify_events};
use super::paths::{is_in_library, mark_indexed_path_missing, rebase_indexed_directory};
use super::{FileWatcher, events, is_audio_file};

impl FileWatcher {
    pub(super) async fn handle_events(
        app: &WatcherContext,
        events: &[DebouncedEvent],
    ) -> Result<(), DatabaseError> {
        let ClassifiedEvents {
            changed,
            removed,
            renamed,
        } = classify_events(events);
        if changed.is_empty() && removed.is_empty() && renamed.is_empty() {
            return Ok(());
        }
        Self::enqueue_events(app, changed, removed, renamed).await
    }

    async fn enqueue_events(
        app_handle: &WatcherContext,
        mut changed: Vec<PathBuf>,
        removed: Vec<PathBuf>,
        renamed: Vec<(PathBuf, PathBuf)>,
    ) -> Result<(), DatabaseError> {
        let state = app_handle;
        let db: &Database = &state.db;
        let roots = get_library_roots(&db.pool)
            .await?
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        let mut changed_ids = Vec::new();
        let mut renamed_fallbacks = Vec::new();
        for (old_path, new_path) in renamed {
            if !new_path.exists() || !is_in_library(&new_path, &roots) {
                changed_ids.extend(mark_indexed_path_missing(&db.pool, &old_path).await?);
                continue;
            }
            if new_path.is_dir() {
                let ids = rebase_indexed_directory(&db.pool, &old_path, &new_path).await?;
                if ids.is_empty() {
                    renamed_fallbacks.push(new_path);
                } else {
                    changed_ids.extend(ids);
                }
                continue;
            }
            if !is_audio_file(&new_path) {
                changed_ids.extend(mark_indexed_path_missing(&db.pool, &old_path).await?);
                continue;
            }
            let metadata = match crate::library_files::metadata(&new_path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    if error.kind() == std::io::ErrorKind::NotFound {
                        changed_ids.extend(mark_indexed_path_missing(&db.pool, &old_path).await?);
                    }
                    continue;
                }
            };
            let modified_at = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0);
            let new_path_string = new_path.to_string_lossy().to_string();
            let file_name = new_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("file");

            let old_id: Option<i64> = sqlx::query_scalar("SELECT id FROM files WHERE path=?")
                .bind(old_path.to_string_lossy().as_ref())
                .fetch_optional(&db.pool)
                .await?;
            if old_id.is_none() {
                renamed_fallbacks.push(new_path);
                continue;
            }
            let replaced_ids = sqlx::query_scalar::<_, i64>(
                "DELETE FROM files WHERE path = ?1 AND path != ?2 RETURNING id",
            )
            .bind(&new_path_string)
            .bind(old_path.to_string_lossy().to_string())
            .fetch_all(&db.pool)
            .await
            .map_err(DatabaseError::Sqlx)?;
            changed_ids.extend(replaced_ids);
            let id = sqlx::query_scalar::<_, i64>(
                "UPDATE files
                 SET path = ?1, file_name = ?2, size = ?3, modified_at = ?4,
                     status = 'pending', metadata_status = 'ok', missing_since = NULL
                 WHERE path = ?5 RETURNING id",
            )
            .bind(&new_path_string)
            .bind(file_name)
            .bind(metadata.len() as i64)
            .bind(modified_at)
            .bind(old_path.to_string_lossy().to_string())
            .fetch_optional(&db.pool)
            .await
            .map_err(DatabaseError::Sqlx)?;
            if let Some(id) = id {
                changed_ids.push(id);
            } else {
                renamed_fallbacks.push(new_path);
            }
        }

        for path in removed {
            if path.exists() {
                changed.push(path);
                continue;
            }
            changed_ids.extend(mark_indexed_path_missing(&db.pool, &path).await?);
        }

        changed.extend(renamed_fallbacks);
        let roots_for_scan = roots.clone();
        let changed = async_runtime::spawn_blocking(move || {
            let mut files = Vec::new();
            for path in changed {
                if !is_in_library(&path, &roots_for_scan) {
                    continue;
                }
                if path.is_dir() {
                    files.extend(
                        WalkDir::new(path)
                            .into_iter()
                            .filter_map(Result::ok)
                            .filter(|entry| entry.file_type().is_file())
                            .map(|entry| entry.into_path())
                            .filter(|path| is_audio_file(path)),
                    );
                } else if is_audio_file(&path) {
                    files.push(path);
                }
            }
            let mut seen = HashSet::new();
            files
                .into_iter()
                .filter(|path| seen.insert(path.clone()))
                .filter_map(|path| {
                    let metadata = crate::library_files::metadata(&path).ok()?;
                    Some((
                        path.to_string_lossy().to_string(),
                        metadata
                            .modified()
                            .ok()?
                            .duration_since(UNIX_EPOCH)
                            .ok()?
                            .as_secs() as i64,
                        metadata.len() as i64,
                    ))
                })
                .collect::<Vec<_>>()
        })
        .await
        .map_err(DatabaseError::Join)?;

        for chunk in changed.chunks(999) {
            let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
                "INSERT INTO files (path, file_name, size, modified_at, status) ",
            );
            query_builder.push_values(chunk, |mut builder, (path, modified_at, size)| {
                let file_name = Path::new(path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file");
                builder
                    .push_bind(path)
                    .push_bind(file_name)
                    .push_bind(*size)
                    .push_bind(*modified_at)
                    .push_bind("pending");
            });
            query_builder.push(
                " ON CONFLICT(path) DO UPDATE SET
                 file_name = excluded.file_name,
                 size = excluded.size,
                 modified_at = excluded.modified_at,
                 status = 'pending', metadata_status = 'ok', missing_since = NULL RETURNING id",
            );
            let ids = query_builder
                .build_query_scalar::<i64>()
                .fetch_all(&db.pool)
                .await
                .map_err(DatabaseError::Sqlx)?;
            changed_ids.extend(ids);
        }

        changed_ids.sort_unstable();
        changed_ids.dedup();
        let mut path_updates = HashMap::new();
        let mut missing_ids = HashSet::new();
        for batch in changed_ids.chunks(500) {
            let mut query = QueryBuilder::<Sqlite>::new(
                "SELECT id, path, missing_since FROM files WHERE id IN (",
            );
            let mut separated = query.separated(",");
            for id in batch {
                separated.push_bind(id);
            }
            separated.push_unseparated(")");
            let rows = query
                .build_query_as::<(i64, String, Option<i64>)>()
                .fetch_all(&db.pool)
                .await
                .map_err(DatabaseError::Sqlx)?;
            let found_ids = rows.iter().map(|row| row.0).collect::<HashSet<_>>();
            for (id, path, missing_since) in rows {
                if missing_since.is_some() {
                    missing_ids.insert(id);
                } else {
                    path_updates.insert(id, path);
                }
            }
            missing_ids.extend(batch.iter().filter(|id| !found_ids.contains(id)).copied());
        }
        app_handle
            .files
            .reconcile(path_updates, missing_ids.into_iter().collect());
        events::emit_changed(app_handle, &changed_ids)?;
        FileWatcher::handle_pending(app_handle).await?;

        Ok(())
    }
}
