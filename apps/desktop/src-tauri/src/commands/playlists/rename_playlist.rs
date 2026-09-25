use super::types::RenamePlaylistInput;
use crate::AppState;
use tauri::command;

#[command]
pub async fn rename_playlist(
    state: tauri::State<'_, AppState>,
    input: RenamePlaylistInput,
) -> Result<(), String> {
    let trimmed = input.name.trim();
    if trimmed.is_empty() {
        return Err("Playlist name is required".to_string());
    }

    sqlx::query("UPDATE playlists SET name = ?1 WHERE id = ?2")
        .bind(trimmed)
        .bind(input.id)
        .execute(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?;

    Ok(())
}
