use super::types::ReorderPlaylistTrackInput;
use crate::commands::playback::shared::emit_queue_changed;
use crate::AppState;
use tauri::{command, AppHandle};

#[command]
pub async fn reorder_playlist_track(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    input: ReorderPlaylistTrackInput,
) -> Result<(), String> {
    let (playlist_id, from, to) = (input.playlist_id, input.from_ord, input.to_ord);
    reorder_tracks(&state.db.pool, input).await?;
    {
        let player = state
            .audio_player
            .lock()
            .map_err(|_| "Audio player is unavailable".to_string())?;
        let mut queue = player
            .queue
            .lock()
            .map_err(|_| "Audio queue is unavailable".to_string())?;
        queue.reorder_playlist_ordinals(playlist_id, from, to);
    }
    emit_queue_changed(&app, &state)
}

async fn reorder_tracks(
    pool: &sqlx::SqlitePool,
    input: ReorderPlaylistTrackInput,
) -> Result<(), String> {
    if input.from_ord < 0 || input.to_ord < 0 {
        return Err("Invalid playlist track position".to_string());
    }
    let mut tx = pool.begin().await.map_err(|error| error.to_string())?;
    let endpoints = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM playlist_tracks
         WHERE playlist_id = ?1 AND ord IN (?2, ?3)",
    )
    .bind(input.playlist_id)
    .bind(input.from_ord)
    .bind(input.to_ord)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;
    let expected = if input.from_ord == input.to_ord { 1 } else { 2 };
    if endpoints != expected {
        return Err("Playlist changed; refresh and try again".to_string());
    }

    sqlx::query(
        "UPDATE playlist_tracks SET ord = -ord - 1
         WHERE playlist_id = ?1 AND ord BETWEEN ?2 AND ?3",
    )
    .bind(input.playlist_id)
    .bind(input.from_ord.min(input.to_ord))
    .bind(input.from_ord.max(input.to_ord))
    .execute(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;

    sqlx::query(
        "UPDATE playlist_tracks
         SET ord = CASE
             WHEN -ord - 1 = ?2 THEN ?3
             WHEN ?2 < ?3 THEN -ord - 2
             ELSE -ord
         END
         WHERE playlist_id = ?1 AND ord BETWEEN ?4 AND ?5",
    )
    .bind(input.playlist_id)
    .bind(input.from_ord)
    .bind(input.to_ord)
    .bind(-input.from_ord.max(input.to_ord) - 1)
    .bind(-input.from_ord.min(input.to_ord) - 1)
    .execute(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;

    tx.commit().await.map_err(|error| error.to_string())
}
