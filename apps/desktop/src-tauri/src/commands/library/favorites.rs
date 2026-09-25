use crate::AppState;

#[tauri::command]
pub async fn get_favorite_ids(state: tauri::State<'_, AppState>) -> Result<Vec<i64>, String> {
    sqlx::query_scalar("SELECT file_id FROM media_info WHERE loved = 1 ORDER BY file_id")
        .fetch_all(&state.db.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_media_loved(
    state: tauri::State<'_, AppState>,
    file_id: i64,
    loved: bool,
) -> Result<(), String> {
    let result = sqlx::query("INSERT INTO media_info (file_id, loved) SELECT id, ? FROM files WHERE id = ? ON CONFLICT(file_id) DO UPDATE SET loved = excluded.loved")
        .bind(loved).bind(file_id).execute(&state.db.pool).await.map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("This song is no longer in your library".into());
    }
    Ok(())
}
