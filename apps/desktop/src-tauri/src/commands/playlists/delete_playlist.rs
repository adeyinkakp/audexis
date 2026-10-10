use crate::AppState;
use tauri::command;

#[command]
pub async fn delete_playlist(
    state: tauri::State<'_, AppState>,
    playlist_id: i64,
) -> Result<(), String> {
    sqlx::query("DELETE FROM playlists WHERE id = ?1")
        .bind(playlist_id)
        .execute(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}
