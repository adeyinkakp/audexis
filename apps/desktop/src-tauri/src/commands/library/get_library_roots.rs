use crate::AppState;
#[tauri::command]
pub async fn get_library_roots(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    sqlx::query_scalar("SELECT path FROM import_roots ORDER BY path")
        .fetch_all(&state.db.pool)
        .await
        .map_err(|error| error.to_string())
}
