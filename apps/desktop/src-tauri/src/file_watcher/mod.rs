mod backfill;
mod events;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, UNIX_EPOCH};

use notify::{event::ModifyKind, EventKind, RecommendedWatcher, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};

use sqlx::{QueryBuilder, Sqlite};
use symphonia::core::formats::{probe::Hint, TrackType};
use symphonia::core::io::MediaSourceStream;
use tauri::{async_runtime, AppHandle, Manager};
use walkdir::WalkDir;

use crate::database::Database;
use crate::tag_manager::tag_backend::{DefaultBackend, TagBackend};
use crate::tag_manager::utils::{MetadataFile, TagValue};
use crate::utils::database::get_library_roots;
use crate::utils::errors::DatabaseError;
use crate::AppState;

#[derive(Debug)]
pub struct FileWatcher {
    debouncer: Debouncer<RecommendedWatcher, FileIdMap>,

    app_handle: AppHandle,
}

struct PendingWorkerGuard(Arc<AtomicBool>);

impl Drop for PendingWorkerGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl FileWatcher {
    pub fn new(app: &AppHandle) -> Result<Self, DatabaseError> {
        let app_handle = app.clone();

        let debouncer = new_debouncer(
            Duration::from_millis(500),
            None,
            move |res: Result<Vec<DebouncedEvent>, _>| match res {
                Ok(events) => {
                    FileWatcher::handle_events(&app_handle, &events);
                }
                Err(error) => crate::utils::errors::report_backend_error(
                    &app_handle,
                    "FileWatcherError",
                    "The library watcher encountered an error",
                    format!("{error:?}"),
                ),
            },
        )
        .map_err(|_| DatabaseError::Unknown(String::from("Unknown")))?;
        let new_watcher = Self {
            app_handle: app.clone(),
            debouncer,
        };

        Ok(new_watcher)
    }
    pub async fn init(&mut self) -> Result<(), DatabaseError> {
        let app_handle = self.app_handle.clone();
        let state: tauri::State<'_, AppState> = app_handle.state::<AppState>();
        let db: &Database = &state.db;
        let roots = get_library_roots(&db.pool).await?;

        for root in &roots {
            let pth = Path::new(&root);
            if pth.exists() == false {
                continue;
            }
            if pth.is_file() {
                continue;
            }
            if let Err(error) = self
                .debouncer
                .watcher()
                .watch(pth, notify::RecursiveMode::Recursive)
            {
                crate::utils::errors::report_backend_error(
                    &self.app_handle,
                    "FileWatcherError",
                    "Could not watch a library folder",
                    format!("{root}: {error}"),
                );
            }
        }

        let scan_app_handle = self.app_handle.clone();
        async_runtime::spawn(async move {
            if let Err(error) = FileWatcher::scan_folders_for_app(&scan_app_handle, roots).await {
                drop(error);
            }
        });

        Ok(())
    }
    pub fn unwatch_folders(&mut self, folders: Vec<String>) {
        for folder in folders {
            let _ = self.debouncer.watcher().unwatch(Path::new(&folder));
        }
    }

    pub fn watch_folders(&mut self, folders: Vec<String>) {
        for root in folders {
            let pth = Path::new(&root);
            if pth.exists() == false {
                continue;
            }
            if pth.is_file() {
                continue;
            }

            let _ = self
                .debouncer
                .watcher()
                .watch(pth, notify::RecursiveMode::Recursive);
        }
    }
    pub fn handle_events(app: &AppHandle, events: &Vec<DebouncedEvent>) {
        let mut changed = Vec::new();
        let mut removed = Vec::new();
        let mut renamed = Vec::new();

        for event in events {
            println!("{:?}", event.kind);

            if matches!(event.kind, EventKind::Modify(ModifyKind::Name(_)))
                && event.paths.len() >= 2
            {
                let old_path = event.paths.first().unwrap().to_path_buf();
                let new_path = event.paths.last().unwrap().to_path_buf();
                match (is_audio_file(&old_path), is_audio_file(&new_path)) {
                    (true, true) => renamed.push((old_path, new_path)),
                    (true, false) => removed.push(old_path),
                    (false, true) => changed.push(new_path),
                    (false, false) => {}
                }
                continue;
            }

            for path in &event.paths {
                let path = path.to_path_buf();
                match event.kind {
                    EventKind::Remove(_) if is_audio_file(&path) => removed.push(path),
                    EventKind::Create(_) | EventKind::Modify(_) if is_audio_file(&path) => {
                        changed.push(path)
                    }
                    _ => {}
                }
            }
        }

        if changed.is_empty() && removed.is_empty() && renamed.is_empty() {
            return;
        }

        let app_handle = app.clone();
        async_runtime::spawn(async move {
            if let Err(error) =
                FileWatcher::enqueue_events(&app_handle, changed, removed, renamed).await
            {
                tauri_plugin_log::log::error!("Could not process file watcher events: {error}");
            }
        });
    }

    async fn enqueue_events(
        app_handle: &AppHandle,
        mut changed: Vec<PathBuf>,
        removed: Vec<PathBuf>,
        renamed: Vec<(PathBuf, PathBuf)>,
    ) -> Result<(), DatabaseError> {
        let state: tauri::State<'_, AppState> = app_handle.state::<AppState>();
        let db: &Database = &state.db;

        let mut changed_ids = Vec::new();
        let mut renamed_fallbacks = Vec::new();
        for (old_path, new_path) in renamed {
            let Ok(metadata) = std::fs::metadata(&new_path) else {
                continue;
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
            let id = sqlx::query_scalar::<_, i64>(
                "UPDATE files
                 SET path = ?1, file_name = ?2, size = ?3, modified_at = ?4,
                     status = 'pending', metadata_status = 'ok'
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
            let ids =
                sqlx::query_scalar::<_, i64>("DELETE FROM files WHERE path = ?1 RETURNING id")
                    .bind(path.to_string_lossy().to_string())
                    .fetch_all(&db.pool)
                    .await
                    .map_err(DatabaseError::Sqlx)?;
            changed_ids.extend(ids);
        }

        changed.extend(renamed_fallbacks);
        let changed = async_runtime::spawn_blocking(move || {
            changed
                .into_iter()
                .filter_map(|path| {
                    let metadata = std::fs::metadata(&path).ok()?;
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
        .map_err(DatabaseError::Tauri)?;

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
                 status = 'pending' RETURNING id",
            );
            let ids = query_builder
                .build_query_scalar::<i64>()
                .fetch_all(&db.pool)
                .await
                .map_err(DatabaseError::Sqlx)?;
            changed_ids.extend(ids);
        }

        events::emit_changed(app_handle, &changed_ids)?;

        let app_handle = app_handle.clone();
        async_runtime::spawn(async move {
            if let Err(error) = FileWatcher::handle_pending(&app_handle).await {
                tauri_plugin_log::log::error!("Could not process pending files: {error}");
            }
        });

        Ok(())
    }
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

                    let Ok(meta) = entry.metadata() else { continue };
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
                    if discovered % 50 == 0 {
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
        .map_err(|err| DatabaseError::Sqlx(err))?;

        let mut batch = Vec::with_capacity(300);
        while let Some(item) = scan_rx.recv().await {
            batch.push(item);
            if batch.len() >= 300 {
                FileWatcher::flush_batch(&mut connection, &mut batch)
                    .await
                    .map_err(|err| DatabaseError::Sqlx(err))?;
            }
        }
        FileWatcher::flush_batch(&mut connection, &mut batch)
            .await
            .map_err(|err| DatabaseError::Sqlx(err))?;
        walk_handle.await.map_err(DatabaseError::Tauri)?;

        let mut changed_ids = sqlx::query_scalar::<_, i64>(
            "INSERT INTO files (path, file_name, size, modified_at, status)
             SELECT s.path, s.file_name, s.size, s.modified_at, 'pending'
             FROM scan_results s
             LEFT JOIN files f ON f.path = s.path
             WHERE f.id IS NULL
                OR f.modified_at != s.modified_at
                OR f.size != s.size
             ON CONFLICT(path) DO UPDATE SET
                file_name = excluded.file_name,
                size = excluded.size,
                modified_at = excluded.modified_at,
                status = 'pending' RETURNING id",
        )
        .fetch_all(&mut *connection)
        .await
        .map_err(DatabaseError::Sqlx)?;

        let deleted_ids = sqlx::query_scalar::<_, i64>(
            "DELETE FROM files
             WHERE path NOT IN (SELECT path FROM scan_results) RETURNING id",
        )
        .fetch_all(&mut *connection)
        .await
        .map_err(DatabaseError::Sqlx)?;

        sqlx::query("DROP TABLE scan_results")
            .execute(&mut *connection)
            .await
            .map_err(|err| DatabaseError::Sqlx(err))?;

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

        let app_handle = app_handle.clone();
        async_runtime::spawn(async move {
            if let Err(error) = FileWatcher::handle_pending(&app_handle).await {
                drop(error);
            }
        });

        task.complete(format!("Library scan complete · {changed_count} changes"));
        Ok(())
    }

    async fn handle_pending(app_handle: &AppHandle) -> Result<(), DatabaseError> {
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
            "SELECT COUNT(*) FROM files WHERE status = 'pending' AND metadata_status = 'ok'",
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
                 WHERE status = 'pending' AND metadata_status = 'ok'
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

    async fn store_metadata(
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

fn read_duration_ms(path: &Path) -> Option<i64> {
    let source = std::fs::File::open(path).ok()?;
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

fn is_audio_file(p: &Path) -> bool {
    const SUPPORTED_EXTENSIONS: [&str; 15] = [
        "m4a", "mp4", "qt", "m4b", "m4v", "mov", "ogg", "opus", "oga", "spx", "ogv", "mp3", "mp2",
        "mp1", "flac",
    ];
    p.extension().is_some_and(|v| {
        v.to_str()
            .is_some_and(|s| SUPPORTED_EXTENSIONS.contains(&&s))
    })
}
// fn systemtime_to_unix(time: SystemTime) -> i64 {
//     time.duration_since(UNIX_EPOCH)
//         .unwrap_or(Duration::from_secs(0))
//         .as_secs() as i64
// }
