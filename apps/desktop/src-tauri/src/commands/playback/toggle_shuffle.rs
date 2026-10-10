use tauri::command;

use crate::AppState;

use super::shared::{emit_playback_modes_changed, emit_queue_changed};

#[command]
pub fn toggle_shuffle(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    let mut queue = player
        .queue
        .lock()
        .map_err(|_| "Audio queue is unavailable".to_string())?;
    queue.toggle_shuffle();
    let _ = queue.preload();
    drop(queue);
    drop(player);
    emit_queue_changed(&app, &state)?;
    emit_playback_modes_changed(&app, &state)
}
