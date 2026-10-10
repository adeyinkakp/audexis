use tauri::{command, AppHandle};

use crate::AppState;

use super::shared::{emit_queue_changed, load_now_playing, publish_now_playing, PlayBackInfo};

#[command]
pub async fn play_song(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    playback_info: PlayBackInfo,
) -> Result<(), String> {
    let info = load_now_playing(&state, &playback_info.curr.path).await?;
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    let mut queue = player
        .queue
        .lock()
        .map_err(|_| "Audio queue is unavailable".to_string())?;

    let mut queue_paths = playback_info.queue;
    let clicked_position = playback_info.current_index.unwrap_or_else(|| {
        queue_paths
            .iter()
            .position(|item| {
                item.id == playback_info.curr.id
                    && item.path == playback_info.curr.path
                    && item.occurrence == playback_info.curr.occurrence
            })
            .or_else(|| {
                queue_paths.iter().position(|item| {
                    item.id == playback_info.curr.id && item.path == playback_info.curr.path
                })
            })
            .unwrap_or(0)
    });

    if queue_paths.get(clicked_position).is_none() {
        queue_paths = vec![playback_info.curr.clone()];
        queue.replace_with(queue_paths, 0);
    } else {
        queue.replace_with(queue_paths, clicked_position);
    }
    queue.playlist_id = playback_info.playlist_id;
    drop(queue);
    player.restart();
    drop(player);

    emit_queue_changed(&app, &state)?;
    publish_now_playing(&app, &state, info)
}
