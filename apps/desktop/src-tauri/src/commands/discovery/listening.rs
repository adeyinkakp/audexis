use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

#[derive(Serialize, FromRow)]
pub struct SongTotal {
    pub file_id: i64,
    pub plays: i64,
}
#[derive(Serialize, FromRow)]
pub struct CollectionTotal {
    pub name: String,
    pub artist: String,
    pub file_id: i64,
    pub plays: i64,
}
#[derive(Serialize, FromRow)]
pub struct RecentItem {
    pub kind: String,
    pub name: String,
    pub artist: String,
    pub file_id: i64,
    pub playlist_id: Option<i64>,
    pub played_at: i64,
}
#[derive(Serialize)]
pub struct Listening {
    pub most_songs: Vec<SongTotal>,
    pub most_albums: Vec<CollectionTotal>,
    pub most_artists: Vec<CollectionTotal>,
    pub recent_items: Vec<RecentItem>,
}
const TAGGED: &str = "tagged AS (SELECT f.id,
 COALESCE((SELECT value FROM metadata_texts WHERE file_id=f.id AND key='album' AND TRIM(value)<>'' ORDER BY ord LIMIT 1),'') AS album,
 COALESCE((SELECT value FROM metadata_texts WHERE file_id=f.id AND key IN ('albumArtist','artist') AND TRIM(value)<>'' ORDER BY CASE key WHEN 'albumArtist' THEN 0 ELSE 1 END, ord LIMIT 1),'') AS owner,
 COALESCE((SELECT value FROM metadata_texts WHERE file_id=f.id AND key IN ('artist','albumArtist') AND TRIM(value)<>'' ORDER BY CASE key WHEN 'artist' THEN 0 ELSE 1 END, ord LIMIT 1),'') AS artist FROM files f)";

pub async fn read(
    pool: &SqlitePool,
    since: Option<i64>,
    until: Option<i64>,
) -> Result<Listening, String> {
    if since.is_some_and(|v| v < 0)
        || until.is_some_and(|v| v < 0)
        || matches!((since,until),(Some(a),Some(b)) if a>=b)
    {
        return Err("Invalid listening-history date range".into());
    }
    let counts = if since.is_none() && until.is_none() {
        "counts AS (SELECT file_id, plays FROM media_info WHERE plays>0 AND (? IS NULL) AND (? IS NULL))"
    } else {
        "counts AS (SELECT file_id,COUNT(*) AS plays FROM counted_plays WHERE played_at IS NOT NULL AND played_at>=COALESCE(?,0) AND played_at<COALESCE(?,9223372036854775807) GROUP BY file_id)"
    };
    let prefix = format!("WITH {counts}, {TAGGED}");
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let most_songs = sqlx::query_as::<_,SongTotal>(&format!("{prefix} SELECT c.file_id,c.plays FROM counts c JOIN files f ON f.id=c.file_id ORDER BY plays DESC,file_id LIMIT 20"))
        .bind(since).bind(until).fetch_all(&mut *tx).await.map_err(|e|e.to_string())?;
    let most_albums = sqlx::query_as::<_,CollectionTotal>(&format!("{prefix} SELECT t.album AS name,t.owner AS artist,MIN(t.id) AS file_id,SUM(c.plays) AS plays FROM counts c JOIN tagged t ON t.id=c.file_id WHERE t.album<>'' GROUP BY t.album COLLATE NOCASE,t.owner COLLATE NOCASE ORDER BY plays DESC,name,artist LIMIT 20"))
        .bind(since).bind(until).fetch_all(&mut *tx).await.map_err(|e|e.to_string())?;
    let most_artists = sqlx::query_as::<_,CollectionTotal>(&format!("{prefix} SELECT t.artist AS name,'' AS artist,MIN(t.id) AS file_id,SUM(c.plays) AS plays FROM counts c JOIN tagged t ON t.id=c.file_id WHERE t.artist<>'' GROUP BY t.artist COLLATE NOCASE ORDER BY plays DESC,name LIMIT 20"))
        .bind(since).bind(until).fetch_all(&mut *tx).await.map_err(|e|e.to_string())?;
    let recent_items = sqlx::query_as::<_, RecentItem>(&format!("WITH {TAGGED}, recent AS (
        SELECT 'song' AS kind, '' AS name, '' AS artist, h.file_id, NULL AS playlist_id, MAX(h.played_at) AS played_at
        FROM listening_history h JOIN files f ON f.id=h.file_id GROUP BY h.file_id
        UNION ALL
        SELECT 'album', t.album, t.owner, MIN(t.id), NULL, MAX(h.played_at)
        FROM listening_history h JOIN tagged t ON t.id=h.file_id WHERE t.album<>'' GROUP BY t.album COLLATE NOCASE,t.owner COLLATE NOCASE
        UNION ALL
        SELECT 'playlist', p.name, '', MIN(h.file_id), p.id, MAX(h.played_at)
        FROM listening_history h JOIN playlists p ON p.id=h.playlist_id GROUP BY p.id,p.name
    ) SELECT kind,name,artist,file_id,playlist_id,played_at FROM recent ORDER BY played_at DESC,kind,name,file_id LIMIT 30"))
        .fetch_all(&mut *tx).await.map_err(|e|e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(Listening {
        most_songs,
        most_albums,
        most_artists,
        recent_items,
    })
}
