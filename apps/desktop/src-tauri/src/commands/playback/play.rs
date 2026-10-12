use tauri::{command, AppHandle};

use crate::AppState;

use super::shared::{emit_queue_changed, load_now_playing, publish_now_playing, PlayBackInfo};

#[command]
pub async fn play_song(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    playback_info: PlayBackInfo,
) -> Result<(), String> {
    let (mut queue_paths, clicked_position) = if let Some(source) = playback_info.source {
        let tracks =
            crate::commands::library::search_media::playback_queue(&state.db.pool, source).await?;
        let index = tracks
            .iter()
            .position(|track| track.id == playback_info.curr.id)
            .ok_or("This song is no longer in the selected collection")?;
        (tracks, index)
    } else {
        let tracks = playback_info.queue;
        let index = playback_info.current_index.unwrap_or_else(|| {
            tracks
                .iter()
                .position(|item| {
                    item.id == playback_info.curr.id
                        && item.path == playback_info.curr.path
                        && item.occurrence == playback_info.curr.occurrence
                })
                .or_else(|| {
                    tracks.iter().position(|item| {
                        item.id == playback_info.curr.id && item.path == playback_info.curr.path
                    })
                })
                .unwrap_or(0)
        });
        (tracks, index)
    };
    let path = queue_paths
        .get(clicked_position)
        .map(|track| track.path.as_str())
        .unwrap_or(&playback_info.curr.path);
    let info = load_now_playing(&state, path).await?;
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    let mut queue = player
        .queue
        .lock()
        .map_err(|_| "Audio queue is unavailable".to_string())?;

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
