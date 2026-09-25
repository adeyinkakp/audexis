use super::{shared::emit_queue_changed, skip_to_index::skip_to_index};
use crate::{audio_player::queue_track::UnresolvedTrack, AppState};

#[tauri::command]
pub async fn enqueue_song(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    file_id: i64,
    next: bool,
) -> Result<(), String> {
    let path = sqlx::query_scalar::<_, String>("SELECT path FROM files WHERE id = ?")
        .bind(file_id)
        .fetch_one(&state.db.pool)
        .await
        .map_err(|e| e.to_string())?;
    let start = {
        let player = state
            .audio_player
            .lock()
            .map_err(|_| "Audio player is unavailable")?;
        let mut queue = player
            .queue
            .lock()
            .map_err(|_| "Audio queue is unavailable")?;
        let empty = queue.tracks.is_empty();
        let item = UnresolvedTrack {
            id: file_id,
            path,
            occurrence: None,
        };
        if next {
            queue.insert_next(item);
        } else {
            queue.append(item);
        }
        empty
    };
    if start {
        skip_to_index(app, state, 0).await
    } else {
        emit_queue_changed(&app, &state)
    }
}
