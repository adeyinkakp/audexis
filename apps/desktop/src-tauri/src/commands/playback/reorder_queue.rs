use super::shared::emit_queue_changed;
use crate::AppState;
use tauri::AppHandle;

#[tauri::command]
pub fn reorder_queue(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    queue_id: String,
    target_id: String,
) -> Result<(), String> {
    {
        let player = state.audio_player.lock().map_err(|_| "Audio player is unavailable")?;
        let mut queue = player.queue.lock().map_err(|_| "Audio queue is unavailable")?;
        queue.reorder(&queue_id, &target_id)?;
    }
    emit_queue_changed(&app, &state)
}
