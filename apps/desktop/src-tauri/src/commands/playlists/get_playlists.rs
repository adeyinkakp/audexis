use super::types::PlaylistSummary;
use crate::AppState;
use tauri::command;

#[command]
pub async fn get_playlists(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PlaylistSummary>, String> {
    sqlx::query_as::<_, PlaylistSummary>(
        "SELECT p.id, p.name, COUNT(pt.file_id) AS track_count,
         (SELECT pt2.file_id FROM playlist_tracks pt2 WHERE pt2.playlist_id = p.id
          AND EXISTS (SELECT 1 FROM metadata_pictures mp WHERE mp.file_id = pt2.file_id)
          ORDER BY pt2.ord LIMIT 1) AS artwork_file_id
         FROM playlists p
         LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
         GROUP BY p.id, p.name
         ORDER BY LOWER(p.name), p.id",
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|error| error.to_string())
}
