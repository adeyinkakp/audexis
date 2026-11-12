use serde::Serialize;
use sqlx::FromRow;

#[derive(Clone, Debug, PartialEq, Serialize, FromRow)]
pub struct SongTime {
    pub file_id: i64,
    pub listened_us: i64,
    pub plays: i64,
}
#[derive(Clone, Debug, PartialEq, Serialize, FromRow)]
pub struct ArtistTime {
    pub name: String,
    pub file_id: i64,
    pub listened_us: i64,
}
#[derive(Clone, Debug, PartialEq, Serialize, FromRow)]
pub struct AlbumTime {
    pub name: String,
    pub artist: String,
    pub file_id: i64,
    pub listened_us: i64,
}
#[derive(Clone, Debug, PartialEq, Serialize, FromRow)]
pub struct PlaylistTime {
    pub playlist_id: i64,
    pub name: String,
    pub file_id: i64,
    pub listened_us: i64,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Rewind {
    pub years: Vec<i64>,
    pub total_us: i64,
    pub artists: Vec<ArtistTime>,
    pub songs: Vec<SongTime>,
    pub albums: Vec<AlbumTime>,
    pub playlists: Vec<PlaylistTime>,
}
const TOTALS: &str = "WITH totals AS (SELECT d.file_id,SUM(d.heard_us) AS listened_us FROM listening_time_daily d JOIN files f ON f.id=d.file_id WHERE f.missing_since IS NULL AND d.day>=? AND d.day<? GROUP BY d.file_id)";
const PLAYS: &str = "WITH plays AS (
    SELECT p.file_id,COUNT(*) AS plays FROM counted_plays p
    JOIN files f ON f.id=p.file_id
    WHERE f.missing_since IS NULL
      AND date(p.played_at,'unixepoch','localtime')>=?
      AND date(p.played_at,'unixepoch','localtime')<?
    GROUP BY p.file_id
)";

pub async fn read(
    pool: &sqlx::SqlitePool,
    year: i32,
    month: Option<u32>,
) -> Result<Rewind, String> {
    let (start, end) = audexis_protocol::ListeningPeriod { year, month }.bounds()?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let years = sqlx::query_scalar("SELECT DISTINCT CAST(substr(d.day,1,4) AS INTEGER) FROM listening_time_daily d JOIN files f ON f.id=d.file_id WHERE f.missing_since IS NULL ORDER BY 1 DESC")
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
    let songs = sqlx::query_as::<_, SongTime>(&format!(
        "{PLAYS}, times AS (
            SELECT file_id,SUM(heard_us) AS listened_us FROM listening_time_daily
            WHERE day>=? AND day<? GROUP BY file_id
         ) SELECT p.file_id,p.plays,COALESCE(t.listened_us,0) AS listened_us
         FROM plays p LEFT JOIN times t ON t.file_id=p.file_id
         ORDER BY p.plays DESC,p.file_id LIMIT 20"
    ))
    .bind(&start)
    .bind(&end)
    .bind(&start)
    .bind(&end)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let albums = sqlx::query_as::<_,AlbumTime>(&format!("{TOTALS}, tagged AS (SELECT t.*,
        (SELECT TRIM(value) FROM metadata_texts WHERE file_id=t.file_id AND key='album' AND TRIM(value)<>'' ORDER BY ord LIMIT 1) AS name,
        COALESCE((SELECT TRIM(value) FROM metadata_texts WHERE file_id=t.file_id AND key='artist' AND TRIM(value)<>'' ORDER BY ord LIMIT 1),'') AS artist FROM totals t)
        SELECT MIN(name) AS name,MIN(artist) AS artist,MIN(file_id) AS file_id,SUM(listened_us) AS listened_us FROM tagged WHERE name IS NOT NULL
        GROUP BY name COLLATE METADATA_NOCASE,artist COLLATE METADATA_NOCASE ORDER BY listened_us DESC,name,artist LIMIT 20"))
        .bind(&start).bind(&end).fetch_all(&mut *tx).await.map_err(|e|e.to_string())?;
    let playlists = sqlx::query_as::<_,PlaylistTime>("SELECT p.id AS playlist_id,p.name,MIN(d.file_id) AS file_id,SUM(d.heard_us) AS listened_us FROM listening_time_daily d JOIN files f ON f.id=d.file_id JOIN library_playlists p ON p.id=d.source_playlist_id WHERE f.missing_since IS NULL AND d.day>=? AND d.day<? GROUP BY p.id,p.name ORDER BY listened_us DESC,p.id LIMIT 20")
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

pub async fn playlist_tracks(
    pool: &sqlx::SqlitePool,
    year: i32,
    month: Option<u32>,
) -> Result<Vec<i64>, String> {
    let (start, end) = audexis_protocol::ListeningPeriod { year, month }.bounds()?;
    sqlx::query_scalar(&format!(
        "{PLAYS} SELECT p.file_id FROM plays p JOIN files f ON f.id=p.file_id
         WHERE f.cloud_connection_id IS NULL OR
               f.cloud_connection_id=(SELECT connection_id FROM cloud_library_session WHERE slot=1)
         ORDER BY p.plays DESC,p.file_id LIMIT 10000"
    ))
    .bind(&start)
    .bind(&end)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())
}

impl super::Database {
    pub async fn create_rewind_playlist(
        &self,
        year: i32,
        month: Option<u32>,
        cloud: bool,
    ) -> Result<(i64, String), String> {
        let name = audexis_protocol::ListeningPeriod { year, month }.playlist_name()?;
        let tracks = playlist_tracks(&self.pool, year, month).await?;
        if tracks.is_empty() {
            return Err("No counted plays for this period yet.".into());
        }
        let id = self
            .create_playlist_with_tracks(&name, &tracks, cloud)
            .await?;
        Ok((id, name))
    }
}
