use super::types::{PlaylistDetail, PlaylistTrack};
use crate::AppState;
use tauri::command;

#[command]
pub async fn get_playlist(
    state: tauri::State<'_, AppState>,
    playlist_id: i64,
) -> Result<PlaylistDetail, String> {
    let playlist =
        sqlx::query_as::<_, (i64, String)>("SELECT id, name FROM playlists WHERE id = ?1")
            .bind(playlist_id)
            .fetch_optional(&state.db.pool)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Playlist not found".to_string())?;

    let tracks = sqlx::query_as::<_, PlaylistTrack>(
        "SELECT f.id, f.path, f.file_name, f.duration_ms, pt.ord
         FROM playlist_tracks pt
         JOIN files f ON f.id = pt.file_id
         WHERE pt.playlist_id = ?1
         ORDER BY pt.ord",
    )
    .bind(playlist_id)
    .fetch_all(&state.db.pool)
    .await
    .map_err(|error| error.to_string())?;

    Ok(PlaylistDetail {
        id: playlist.0,
        name: playlist.1,
        tracks,
    })
}
