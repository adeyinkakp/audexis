use tauri::command;

use crate::{audio_player::PlaybackModes, AppState};

#[command]
pub fn get_playback_modes(state: tauri::State<'_, AppState>) -> Result<PlaybackModes, String> {
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    let queue = player
        .queue
        .lock()
        .map_err(|_| "Audio queue is unavailable".to_string())?;
    Ok(queue.playback_modes())
}
