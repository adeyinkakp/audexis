use crate::{file_watcher::FileWatcher, AppState};
#[tauri::command]
pub async fn rescan_library(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let roots = sqlx::query_scalar("SELECT path FROM import_roots")
        .fetch_all(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?;
    FileWatcher::scan_folders_for_app(&app, roots)
        .await
        .map_err(|error| error.to_string())
}
