use super::shared::{
    emit_playback_modes_changed, emit_queue_changed, load_now_playing, publish_now_playing,
    NowPlayingInfo,
};
use crate::AppState;
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub async fn remove_queue_entries(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    queue_ids: Vec<String>,
) -> Result<(), String> {
    if queue_ids.is_empty() {
        return Ok(());
    }
    let (removed_current, next_path, empty) = {
        let player = state
            .audio_player
            .lock()
            .map_err(|_| "Audio player is unavailable")?;
        let mut queue = player
            .queue
            .lock()
            .map_err(|_| "Audio queue is unavailable")?;
        let removed_current = queue.remove_queue_entries(&queue_ids)?;
        let next_path = queue.current_path();
        let empty = queue.tracks.is_empty();
        drop(queue);
        if removed_current {
            if next_path.is_some() {
                player.restart();
            } else {
                player.stop();
            }
        }
        (removed_current, next_path, empty)
    };
    emit_queue_changed(&app, &state)?;
    if empty {
        emit_playback_modes_changed(&app, &state)?;
    }
    if removed_current {
        if let Some(path) = next_path {
            let info = load_now_playing(&state, &path).await?;
            publish_now_playing(&app, &state, info)?;
        } else {
            *state
                .now_playing
                .lock()
                .map_err(|_| "Now playing state is unavailable")? = None;
            app.emit("now-playing-changed", Option::<NowPlayingInfo>::None)
                .map_err(|error| error.to_string())?;
            app.emit("playback-queue-done", ())
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}
