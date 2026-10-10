use super::shared::{emit_queue_changed, load_now_playing, publish_now_playing};
use crate::AppState;
use tauri::{command, AppHandle};

#[command]
pub async fn skip_to_index(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    index: usize,
) -> Result<(), String> {
    let (queue_id, path) = {
        let player = state
            .audio_player
            .lock()
            .map_err(|_| "Audio player is unavailable".to_string())?;
        let queue = player
            .queue
            .lock()
            .map_err(|_| "Audio queue is unavailable".to_string())?;
        let track = queue
            .tracks
            .get(index)
            .ok_or_else(|| "Queue index is out of range".to_string())?
            .lock()
            .map_err(|_| "Queue track is unavailable".to_string())?;
        (track.queue_id.clone(), track.path.clone())
    };
    let info = load_now_playing(&state, &path).await?;
    {
        let player = state
            .audio_player
            .lock()
            .map_err(|_| "Audio player is unavailable".to_string())?;
        let mut queue = player
            .queue
            .lock()
            .map_err(|_| "Audio queue is unavailable".to_string())?;

        let position = queue
            .tracks
            .iter()
            .position(|track| {
                track
                    .lock()
                    .ok()
                    .is_some_and(|track| track.queue_id == queue_id)
            })
            .ok_or_else(|| "Selected song is no longer in the queue".to_string())?;
        queue.index = position as i32;
        drop(queue);
        player.restart();
    }
    emit_queue_changed(&app, &state)?;
    publish_now_playing(&app, &state, info)
}
