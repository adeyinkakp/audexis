use std::path::Path;
use std::time::UNIX_EPOCH;

use sqlx::{QueryBuilder, Sqlite};
use tauri::{async_runtime, AppHandle, Emitter, Manager};
use walkdir::WalkDir;

use crate::database::Database;
use crate::utils::database::get_library_roots;
use crate::utils::errors::DatabaseError;
use crate::AppState;

use super::{backfill, events, is_audio_file, FileWatcher};

impl FileWatcher {
    pub async fn scan_folders(&self, roots: Vec<String>) -> Result<(), DatabaseError> {
        Self::scan_folders_for_app(&self.app_handle, roots).await
    }

    pub(crate) async fn scan_folders_for_app(
        app_handle: &AppHandle,
        _roots: Vec<String>,
    ) -> Result<(), DatabaseError> {
        let state: tauri::State<'_, AppState> = app_handle.state::<AppState>();
        let db: &Database = &state.db;

        let _scan_guard = state.library_scan_lock.lock().await;
        let task = crate::utils::tasks::TaskGuard::start(
            app_handle,
            "library-scan",
            "Scanning music library",
            None,
        );

        let roots = get_library_roots(&db.pool).await?;
        if let Some(missing) = roots.iter().find(|root| !Path::new(root).is_dir()) {
            return Err(DatabaseError::Unknown(format!(
                "Folder is unavailable: {missing}"
            )));
        }

        let (scan_tx, mut scan_rx) = async_runtime::channel::<(String, String, i64, i64)>(1000);
        let progress_app = app_handle.clone();
        let walk_handle = async_runtime::spawn_blocking(move || {
            let mut discovered = 0_u64;
            for root in roots {
                for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
                    if !entry.file_type().is_file() || !is_audio_file(entry.path()) {
                        continue;
                    }

                    let Ok(meta) = crate::utils::library_files::metadata(entry.path()) else {
                        continue;
                    };
                    let modified_at = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let size = meta.len() as i64;

                    if scan_tx
                        .blocking_send((
                            entry.path().to_string_lossy().to_string(),
                            entry.file_name().to_string_lossy().to_string(),
                            modified_at,
                            size,
                        ))
                        .is_err()
                    {
                        return;
                    }
                    discovered += 1;
                    if discovered.is_multiple_of(50) {
                        crate::utils::tasks::update(
                            &progress_app,
                            "library-scan",
                            discovered,
                            None,
                            Some(format!("Found {discovered} audio files")),
                        );
                    }
                }
            }
            crate::utils::tasks::update(
                &progress_app,
                "library-scan",
                discovered,
                Some(discovered),
                Some(format!("Found {discovered} audio files")),
            );
        });

        let mut connection = db.pool.acquire().await.map_err(DatabaseError::Sqlx)?;
        sqlx::query(
            "CREATE TEMP TABLE scan_results
             (path TEXT PRIMARY KEY, file_name TEXT, modified_at INTEGER, size INTEGER)",
        )
        .execute(&mut *connection)
        .await
        .map_err(DatabaseError::Sqlx)?;

        let mut batch = Vec::with_capacity(300);
        while let Some(item) = scan_rx.recv().await {
            batch.push(item);
            if batch.len() >= 300 {
                FileWatcher::flush_batch(&mut connection, &mut batch)
                    .await
                    .map_err(DatabaseError::Sqlx)?;
            }
        }
        FileWatcher::flush_batch(&mut connection, &mut batch)
            .await
            .map_err(DatabaseError::Sqlx)?;
        walk_handle.await.map_err(DatabaseError::Tauri)?;

        let mut changed_ids = sqlx::query_scalar::<_, i64>(
            "INSERT INTO files (path, file_name, size, modified_at, status)
             SELECT s.path, s.file_name, s.size, s.modified_at, 'pending'
             FROM scan_results s
             LEFT JOIN files f ON f.path = s.path
             WHERE f.id IS NULL
                OR f.modified_at != s.modified_at
                OR f.size != s.size
                OR f.missing_since IS NOT NULL
             ON CONFLICT(path) DO UPDATE SET
                file_name = excluded.file_name,
                size = excluded.size,
                 modified_at = excluded.modified_at,
                 status = 'pending', missing_since = NULL RETURNING id",
        )
        .fetch_all(&mut *connection)
        .await
        .map_err(DatabaseError::Sqlx)?;

        let deleted_ids = sqlx::query_scalar::<_, i64>(
            "UPDATE files SET missing_since = unixepoch()
             WHERE missing_since IS NULL
               AND path NOT IN (SELECT path FROM scan_results)
             RETURNING id",
        )
        .fetch_all(&mut *connection)
        .await
        .map_err(DatabaseError::Sqlx)?;

        sqlx::query("DROP TABLE scan_results")
            .execute(&mut *connection)
            .await
            .map_err(DatabaseError::Sqlx)?;

        sqlx::query("UPDATE import_roots SET last_scanned = unixepoch()")
            .execute(&mut *connection)
            .await
            .map_err(DatabaseError::Sqlx)?;

        changed_ids.extend(deleted_ids);
        let changed_count = changed_ids.len();
        events::emit_changed(app_handle, &changed_ids)?;

        let backfill_app = app_handle.clone();
        let backfill_pool = db.pool.clone();
        async_runtime::spawn(async move {
            if let Err(error) = backfill::backfill(&backfill_pool, &backfill_app).await {
                drop(error);
            }
        });

        let pending_app = app_handle.clone();
        async_runtime::spawn(async move {
            if let Err(error) = FileWatcher::handle_pending(&pending_app).await {
                drop(error);
            }
        });

        let _ = app_handle.emit("library-scan-completed", ());
        task.complete(format!("Library scan complete · {changed_count} changes"));
        Ok(())
    }

    async fn flush_batch(
        connection: &mut sqlx::pool::PoolConnection<Sqlite>,
        batch: &mut Vec<(String, String, i64, i64)>,
    ) -> Result<(), sqlx::Error> {
        if batch.is_empty() {
            return Ok(());
        }

        let mut query_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("INSERT INTO scan_results (path, file_name, modified_at, size)");

        query_builder.push_values(
            &mut *batch,
            |mut b, (path, file_name, modified_at, size)| {
                b.push_bind(path.to_string())
                    .push_bind(file_name.to_string())
                    .push_bind(modified_at.to_owned())
                    .push_bind(size.to_owned());
            },
        );
        query_builder.build().execute(&mut **connection).await?;

        batch.clear();
        Ok(())
    }
}
