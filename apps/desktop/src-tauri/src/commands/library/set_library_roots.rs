use crate::{file_watcher::FileWatcher, AppState};
use std::path::PathBuf;
use tauri::{command, AppHandle};

#[command]
pub async fn set_library_roots(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    folders: Vec<String>,
) -> Result<(), String> {
    let mut paths: Vec<PathBuf> = folders.into_iter().map(PathBuf::from).collect();
    paths.sort();
    paths.dedup();
    let mut roots: Vec<PathBuf> = Vec::new();
    for path in paths {
        if !path.is_dir() {
            return Err(format!("Folder is unavailable: {}", path.display()));
        }
        if roots.iter().any(|root| path.starts_with(root)) {
            continue;
        }
        roots.push(path);
    }
    let roots: Vec<String> = roots
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect();
    let previous: Vec<String> = sqlx::query_scalar("SELECT path FROM import_roots")
        .fetch_all(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?;
    let mut tx = state
        .db
        .pool
        .begin()
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("DELETE FROM import_roots")
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    for path in &roots {
        sqlx::query("INSERT INTO import_roots (path, last_scanned) VALUES (?1, 0)")
            .bind(path)
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
    }
    tx.commit().await.map_err(|error| error.to_string())?;
    {
        let mut watcher = state
            .file_watcher
            .lock()
            .map_err(|_| "File watcher is unavailable".to_string())?;
        watcher.unwatch_folders(
            previous
                .into_iter()
                .filter(|path| !roots.contains(path))
                .collect(),
        );
        watcher.watch_folders(roots.clone());
    }
    FileWatcher::scan_folders_for_app(&app, roots)
        .await
        .map_err(|error| error.to_string())
}
