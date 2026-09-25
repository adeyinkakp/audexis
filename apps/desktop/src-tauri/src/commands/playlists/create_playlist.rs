use super::types::CreatePlaylistInput;
use crate::AppState;
use tauri::command;

#[command]
pub async fn create_playlist(
    state: tauri::State<'_, AppState>,
    input: CreatePlaylistInput,
) -> Result<i64, String> {
    let trimmed = input.name.trim();
    if trimmed.is_empty() {
        return Err("Playlist name is required".to_string());
    }

    let result = sqlx::query("INSERT INTO playlists (name) VALUES (?1)")
        .bind(trimmed)
        .execute(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?;

    Ok(result.last_insert_rowid())
}
