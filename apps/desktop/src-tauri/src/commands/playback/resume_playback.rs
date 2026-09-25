use tauri::command;

use crate::AppState;

#[command]
pub fn resume_playback(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    player.resume();
    Ok(())
}
