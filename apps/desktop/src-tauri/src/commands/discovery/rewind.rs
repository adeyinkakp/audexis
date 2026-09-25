use crate::AppState;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Serialize, FromRow)]
pub struct SongTime {
    file_id: i64,
    listened_us: i64,
}
#[derive(Serialize, FromRow)]
pub struct ArtistTime {
    name: String,
    file_id: i64,
    listened_us: i64,
}
#[derive(Serialize, FromRow)]
pub struct AlbumTime {
    name: String,
    artist: String,
    file_id: i64,
    listened_us: i64,
}
#[derive(Serialize, FromRow)]
pub struct PlaylistTime {
    playlist_id: i64,
    name: String,
    file_id: i64,
    listened_us: i64,
}
#[derive(Serialize)]
pub struct Rewind {
    years: Vec<i64>,
    total_us: i64,
    artists: Vec<ArtistTime>,
    songs: Vec<SongTime>,
    albums: Vec<AlbumTime>,
    playlists: Vec<PlaylistTime>,
}
const TOTALS: &str = "WITH totals AS (SELECT d.file_id,SUM(d.heard_us) AS listened_us FROM listening_time_daily d JOIN files f ON f.id=d.file_id WHERE d.day>=? AND d.day<? GROUP BY d.file_id)";

#[tauri::command]
pub async fn get_rewind(
    state: tauri::State<'_, AppState>,
    year: i32,
    month: Option<u32>,
) -> Result<Rewind, String> {
    if !(1970..=9998).contains(&year) || month.is_some_and(|v| !(1..=12).contains(&v)) {
        return Err("Choose a valid year and month".into());
    }
    let start = format!("{year:04}-{:02}-01", month.unwrap_or(1));
    let end = match month {
        Some(m) if m < 12 => format!("{year:04}-{:02}-01", m + 1),
        _ => format!("{:04}-01-01", year + 1),
    };
    let mut tx = state.db.pool.begin().await.map_err(|e| e.to_string())?;
    let years = sqlx::query_scalar("SELECT DISTINCT CAST(substr(day,1,4) AS INTEGER) FROM listening_time_daily ORDER BY 1 DESC")
        .fetch_all(&mut *tx).await.map_err(|e|e.to_string())?;
    let total_us = sqlx::query_scalar(&format!(
        "{TOTALS} SELECT COALESCE(SUM(listened_us),0) FROM totals"
    ))
    .bind(&start)
    .bind(&end)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    let artists = sqlx::query_as::<_,ArtistTime>(&format!("{TOTALS}, names AS (
        SELECT file_id,MIN(TRIM(value)) AS name FROM metadata_texts WHERE key='artist' AND TRIM(value)<>''
        GROUP BY file_id,TRIM(value) COLLATE METADATA_NOCASE)
        SELECT MIN(n.name) AS name,MIN(t.file_id) AS file_id,SUM(t.listened_us) AS listened_us
        FROM totals t JOIN names n ON n.file_id=t.file_id GROUP BY n.name COLLATE METADATA_NOCASE
        ORDER BY listened_us DESC,name LIMIT 20"))
        .bind(&start).bind(&end).fetch_all(&mut *tx).await.map_err(|e|e.to_string())?;
    let songs = sqlx::query_as::<_,SongTime>(&format!("{TOTALS} SELECT file_id,listened_us FROM totals ORDER BY listened_us DESC,file_id LIMIT 20"))
        .bind(&start).bind(&end).fetch_all(&mut *tx).await.map_err(|e|e.to_string())?;
    let albums = sqlx::query_as::<_,AlbumTime>(&format!("{TOTALS}, tagged AS (SELECT t.*,
        (SELECT TRIM(value) FROM metadata_texts WHERE file_id=t.file_id AND key='album' AND TRIM(value)<>'' ORDER BY ord LIMIT 1) AS name,
        COALESCE((SELECT TRIM(value) FROM metadata_texts WHERE file_id=t.file_id AND key='artist' AND TRIM(value)<>'' ORDER BY ord LIMIT 1),'') AS artist FROM totals t)
        SELECT MIN(name) AS name,MIN(artist) AS artist,MIN(file_id) AS file_id,SUM(listened_us) AS listened_us FROM tagged WHERE name IS NOT NULL
        GROUP BY name COLLATE METADATA_NOCASE,artist COLLATE METADATA_NOCASE ORDER BY listened_us DESC,name,artist LIMIT 20"))
        .bind(&start).bind(&end).fetch_all(&mut *tx).await.map_err(|e|e.to_string())?;
    let playlists = sqlx::query_as::<_,PlaylistTime>("SELECT p.id AS playlist_id,p.name,MIN(d.file_id) AS file_id,SUM(d.heard_us) AS listened_us FROM listening_time_daily d JOIN files f ON f.id=d.file_id JOIN playlists p ON p.id=d.source_playlist_id WHERE d.day>=? AND d.day<? GROUP BY p.id,p.name ORDER BY listened_us DESC,p.id LIMIT 20")
        .bind(&start).bind(&end).fetch_all(&mut *tx).await.map_err(|e|e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(Rewind {
        years,
        total_us,
        artists,
        songs,
        albums,
        playlists,
    })
}
