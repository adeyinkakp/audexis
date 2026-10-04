use crate::AppState;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Serialize, FromRow)]
pub struct LibrarySummary {
    total_tracks: i64,
    total_size: i64,
    album_count: i64,
    last_scanned: Option<i64>,
}

#[derive(Serialize, FromRow)]
pub struct WatchedFolder {
    path: String,
    track_count: i64,
    total_size: i64,
    last_scanned: i64,
}

#[derive(Serialize, FromRow)]
pub struct NeglectedSong {
    file_id: i64,
    last_played: Option<i64>,
}

#[derive(serde::Serialize)]
pub struct HomeDiscovery {
    recent_ids: Vec<i64>,
    favorite_ids: Vec<i64>,

    heavy_rotation: Vec<super::listening::SongTotal>,
    neglected_songs: Vec<NeglectedSong>,
    summary: LibrarySummary,
    watched_folders: Vec<WatchedFolder>,
    listening: super::listening::Listening,
}
#[tauri::command]
pub async fn get_home_discovery(
    state: tauri::State<'_, AppState>,
    since: Option<i64>,
    until: Option<i64>,
) -> Result<HomeDiscovery, String> {
    let mut tx = state.db.pool.begin().await.map_err(|e| e.to_string())?;
    let recent_ids = sqlx::query_scalar("SELECT id FROM files ORDER BY id DESC LIMIT 5")
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let favorite_ids = sqlx::query_scalar("SELECT f.id FROM files f JOIN media_info i ON i.file_id=f.id WHERE i.loved=1 ORDER BY RANDOM() LIMIT 5")
        .fetch_all(&mut *tx).await.map_err(|e| e.to_string())?;

    let heavy_rotation = sqlx::query_as::<_, super::listening::SongTotal>(
        "SELECT file_id, COUNT(*) AS plays
         FROM counted_plays
         WHERE played_at >= unixepoch('now', 'start of month')
         GROUP BY file_id
         ORDER BY plays DESC, MAX(played_at) DESC, file_id
         LIMIT 6",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let neglected_songs = sqlx::query_as::<_, NeglectedSong>(
        "SELECT f.id AS file_id, MAX(h.played_at) AS last_played
         FROM files f
         LEFT JOIN listening_history h ON h.file_id = f.id
         GROUP BY f.id
         ORDER BY last_played IS NOT NULL, last_played, f.id
         LIMIT 6",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let summary = sqlx::query_as::<_, LibrarySummary>(
        "SELECT
            (SELECT COUNT(*) FROM files) AS total_tracks,
            (SELECT COALESCE(SUM(size), 0) FROM files) AS total_size,
            (SELECT COUNT(*) FROM (
                SELECT album.value,
                       COALESCE((SELECT owner.value FROM metadata_texts owner
                                 WHERE owner.file_id = album.file_id
                                   AND owner.key IN ('albumArtist', 'artist')
                                   AND TRIM(owner.value) <> ''
                                 ORDER BY CASE owner.key WHEN 'albumArtist' THEN 0 ELSE 1 END,
                                          owner.ord LIMIT 1), '') AS artist
                FROM metadata_texts album
                WHERE album.key = 'album' AND TRIM(album.value) <> ''
                GROUP BY album.value COLLATE NOCASE, artist COLLATE NOCASE
            )) AS album_count,
            (SELECT MAX(last_scanned) FROM import_roots) AS last_scanned",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let watched_folders = sqlx::query_as::<_, WatchedFolder>(
        "SELECT r.path,
                COUNT(f.id) AS track_count,
                COALESCE(SUM(f.size), 0) AS total_size,
                r.last_scanned
         FROM import_roots r
         LEFT JOIN files f ON instr(f.path, r.path || '/') = 1
                           OR instr(f.path, r.path || char(92)) = 1
         GROUP BY r.id, r.path, r.last_scanned
         ORDER BY r.path COLLATE NOCASE",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    let listening = super::listening::read(&state.db.pool, since, until).await?;
    Ok(HomeDiscovery {
        recent_ids,
        favorite_ids,

        heavy_rotation,
        neglected_songs,
        summary,
        watched_folders,
        listening,
    })
}
