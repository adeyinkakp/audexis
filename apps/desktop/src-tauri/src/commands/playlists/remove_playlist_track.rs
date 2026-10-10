use super::types::RemovePlaylistTrackInput;
use crate::commands::playback::shared::{
    emit_queue_changed, finish_playback_and_clear_queue, load_now_playing, publish_now_playing,
};
use crate::AppState;
use tauri::{command, AppHandle};

#[command]
pub async fn remove_playlist_track(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    input: RemovePlaylistTrackInput,
) -> Result<(), String> {
    let mut tx = state
        .db
        .pool
        .begin()
        .await
        .map_err(|error| error.to_string())?;

    let deleted = sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND ord = ?2")
        .bind(input.playlist_id)
        .bind(input.ord)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;

    if deleted.rows_affected() == 0 {
        return Err("Playlist track not found".to_string());
    }

    sqlx::query(
        "UPDATE playlist_tracks
         SET ord = ord - 1
         WHERE playlist_id = ?1 AND ord > ?2",
    )
    .bind(input.playlist_id)
    .bind(input.ord)
    .execute(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;

    tx.commit().await.map_err(|error| error.to_string())?;
    let (removed_current, next_path) = {
        let player = state
            .audio_player
            .lock()
            .map_err(|_| "Audio player is unavailable".to_string())?;
        let mut queue = player
            .queue
            .lock()
            .map_err(|_| "Audio queue is unavailable".to_string())?;
        let removed_current = queue.remove_for_playlist(input.playlist_id, input.ord);
        let next_path = queue.current_path();
        drop(queue);
        if removed_current {
            if next_path.is_some() {
                player.restart();
            } else {
                player.stop();
            }
        }
        (removed_current, next_path)
    };
    if removed_current {
        if let Some(path) = next_path {
            let info = load_now_playing(&state, &path).await?;
            publish_now_playing(&app, &state, info)?;
        } else {
            return finish_playback_and_clear_queue(&app, &state);
        }
    }
    emit_queue_changed(&app, &state)
}
