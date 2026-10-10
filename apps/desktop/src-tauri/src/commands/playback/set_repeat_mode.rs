use tauri::command;

use crate::{audio_player::RepeatMode, AppState};

use super::shared::emit_playback_modes_changed;

#[command]
pub fn set_repeat_mode(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    repeat_mode: RepeatMode,
) -> Result<(), String> {
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    let mut queue = player
        .queue
        .lock()
        .map_err(|_| "Audio queue is unavailable".to_string())?;
    queue.set_repeat_mode(repeat_mode);
    drop(queue);
    drop(player);
    emit_playback_modes_changed(&app, &state)
}
