use super::{
    FileWatcher, WatcherContext, WatcherError, backfill, events, get_library_roots, is_audio_file,
};
use sqlx::{QueryBuilder, Sqlite};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

impl FileWatcher {
    pub(super) async fn scan_folders_for_context(
        context: &WatcherContext,
    ) -> Result<(), WatcherError> {
        let _guard = context.library_scan_lock.lock().await;
        let task =
            events::TaskGuard::start(context, "library-scan", "Scanning music library", None);
        let roots: Vec<PathBuf> = get_library_roots(&context.db.pool)
            .await?
            .into_iter()
            .map(PathBuf::from)
            .collect();
        let walk_roots = roots.clone();
        let rows = tokio::task::spawn_blocking(move || {
            let mut rows = Vec::new();
            let mut seen = HashSet::new();
            for root in walk_roots {
                if !root.is_dir() {
                    return Err(WatcherError::Unknown(format!(
                        "Folder is unavailable: {}",
                        root.display()
                    )));
                }
                for entry in walkdir::WalkDir::new(root) {
                    let entry = entry.map_err(|e| WatcherError::Unknown(e.to_string()))?;
                    if !entry.file_type().is_file()
                        || !is_audio_file(entry.path())
                        || !seen.insert(entry.path().to_owned())
                    {
                        continue;
                    }
                    let metadata = entry
                        .metadata()
                        .map_err(|e| WatcherError::Unknown(e.to_string()))?;
                    let modified = metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|t| t.as_secs() as i64)
                        .unwrap_or(0);
                    rows.push((
                        entry.path().to_string_lossy().into_owned(),
                        entry.file_name().to_string_lossy().into_owned(),
                        metadata.len() as i64,
                        modified,
                    ));
                }
            }
            Ok::<_, WatcherError>(rows)
        })
        .await??;
        task.update(
            rows.len() as u64,
            Some(rows.len() as u64),
            Some(format!("Found {} audio files", rows.len())),
        );
        let found: HashSet<_> = rows.iter().map(|r| PathBuf::from(&r.0)).collect();
        let mut tx = context.db.pool.begin().await?;
        let mut changed = Vec::new();
        for chunk in rows.chunks(200) {
            let mut query = QueryBuilder::<Sqlite>::new(
                "INSERT INTO files(path,file_name,size,modified_at,status) ",
            );
            query.push_values(chunk, |mut row, (path, name, size, modified)| {
                row.push_bind(path)
                    .push_bind(name)
                    .push_bind(size)
                    .push_bind(modified)
                    .push_bind("pending");
            });
            query.push(" ON CONFLICT(path) DO UPDATE SET file_name=excluded.file_name, size=excluded.size, modified_at=excluded.modified_at, status='pending', metadata_status='ok', missing_since=NULL, hash=NULL WHERE files.size != excluded.size OR files.modified_at != excluded.modified_at OR files.missing_since IS NOT NULL RETURNING id");
            changed.extend(
                query
                    .build_query_scalar::<i64>()
                    .fetch_all(&mut *tx)
                    .await?,
            );
        }
        let indexed: Vec<(i64, String)> =
            sqlx::query_as("SELECT id,path FROM files WHERE missing_since IS NULL")
                .fetch_all(&mut *tx)
                .await?;
        let mut missing = Vec::new();
        for (id, path) in indexed {
            let path = Path::new(&path);
            if roots.iter().any(|root| path.starts_with(root)) && !found.contains(path) {
                sqlx::query("UPDATE files SET missing_since=unixepoch() WHERE id=?")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                missing.push(id);
            }
        }
        sqlx::query("UPDATE import_roots SET last_scanned=unixepoch()")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        changed.extend(&missing);
        context.files.reconcile(Default::default(), missing);
        events::emit_changed(context, &changed)?;
        task.complete(format!("Library scan complete · {} changes", changed.len()));
        Self::handle_pending(context).await?;
        backfill::backfill(&context.db.pool, context).await?;
        context.files.publish_changed(Vec::new());
        Ok(())
    }
}
