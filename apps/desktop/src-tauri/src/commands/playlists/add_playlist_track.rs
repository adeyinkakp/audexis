use super::types::AddPlaylistTrackInput;
use crate::audio_player::queue_track::UnresolvedTrack;
use crate::commands::playback::shared::emit_queue_changed;
use crate::AppState;
use tauri::{command, AppHandle};

#[command]
pub async fn add_playlist_track(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    input: AddPlaylistTrackInput,
) -> Result<(), String> {
    let path = sqlx::query_scalar::<_, String>("SELECT path FROM files WHERE id = ?1")
        .bind(input.file_id)
        .fetch_one(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?;

    let next_ord = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MAX(ord) FROM playlist_tracks WHERE playlist_id = ?1",
    )
    .bind(input.playlist_id)
    .fetch_one(&state.db.pool)
    .await
    .map_err(|error| error.to_string())?
    .unwrap_or(-1)
        + 1;

    sqlx::query("INSERT INTO playlist_tracks (playlist_id, file_id, ord) VALUES (?1, ?2, ?3)")
        .bind(input.playlist_id)
        .bind(input.file_id)
        .bind(next_ord)
        .execute(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?;

    let appended = {
        let player = state
            .audio_player
            .lock()
            .map_err(|_| "Audio player is unavailable".to_string())?;
        let mut queue = player
            .queue
            .lock()
            .map_err(|_| "Audio queue is unavailable".to_string())?;
        queue.append_for_playlist(
            input.playlist_id,
            UnresolvedTrack {
                id: input.file_id,
                path,
                occurrence: Some(next_ord),
            },
        )
    };
    if appended {
        emit_queue_changed(&app, &state)?;
    }
    Ok(())
}
