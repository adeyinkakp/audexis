use super::Database;
use sqlx::FromRow;

#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct DiscoveryTrack {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct HomeDiscovery {
    pub track_count: i64,
    pub recent: Vec<DiscoveryTrack>,
    pub heavy_rotation: Vec<DiscoveryTrack>,
    pub rediscover: Vec<DiscoveryTrack>,
    pub folders: Vec<WatchedFolder>,
}

#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct WatchedFolder {
    pub path: String,
    pub track_count: i64,
    pub total_size: i64,
}

impl Database {
    pub async fn home_discovery(&self) -> Result<HomeDiscovery, sqlx::Error> {
        let select = "SELECT f.id, f.path,
            COALESCE((SELECT NULLIF(TRIM(value), '') FROM metadata_texts WHERE file_id=f.id AND key='title' ORDER BY ord LIMIT 1), f.file_name) AS title,
            COALESCE((SELECT NULLIF(TRIM(value), '') FROM metadata_texts WHERE file_id=f.id AND key='artist' ORDER BY ord LIMIT 1), 'Unknown artist') AS artist,
            COALESCE((SELECT NULLIF(TRIM(value), '') FROM metadata_texts WHERE file_id=f.id AND key='album' ORDER BY ord LIMIT 1), '') AS album,
            f.duration_ms FROM files f";
        let available = "f.status='ok' AND f.missing_since IS NULL";
        let mut tx = self.pool.begin().await?;
        let track_count =
            sqlx::query_scalar(&format!("SELECT COUNT(*) FROM files f WHERE {available}"))
                .fetch_one(&mut *tx)
                .await?;
        let recent = sqlx::query_as(&format!("{select}
            JOIN (SELECT file_id, MAX(played_at) AS last_played FROM listening_history GROUP BY file_id) h ON h.file_id=f.id
            WHERE {available} ORDER BY h.last_played DESC, f.id DESC LIMIT 6"))
            .fetch_all(&mut *tx).await?;
        let heavy_rotation = sqlx::query_as(&format!("{select}
            JOIN (SELECT file_id, COUNT(*) AS plays, MAX(played_at) AS last_played FROM counted_plays
                WHERE played_at >= unixepoch('now', 'start of month') GROUP BY file_id) h ON h.file_id=f.id
            WHERE {available} ORDER BY h.plays DESC, h.last_played DESC, f.id LIMIT 6"))
            .fetch_all(&mut *tx).await?;
        let rediscover = sqlx::query_as(&format!("{select}
            LEFT JOIN (SELECT file_id, MAX(played_at) AS last_played FROM listening_history GROUP BY file_id) h ON h.file_id=f.id
            WHERE {available} ORDER BY h.last_played IS NOT NULL, h.last_played, f.id LIMIT 6"))
            .fetch_all(&mut *tx).await?;
        let folders = sqlx::query_as("SELECT r.path, COUNT(f.id) AS track_count, COALESCE(SUM(f.size),0) AS total_size
            FROM import_roots r LEFT JOIN files f ON f.status='ok' AND f.missing_since IS NULL
            AND (instr(f.path, rtrim(r.path, '/' || char(92)) || '/')=1 OR instr(f.path, rtrim(r.path, '/' || char(92)) || char(92))=1)
            GROUP BY r.id ORDER BY r.path COLLATE NOCASE")
            .fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(HomeDiscovery {
            track_count,
            recent,
            heavy_rotation,
            rediscover,
            folders,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn discovery_uses_history_and_excludes_unavailable_files() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("src/database/migrations")
            .run(&pool)
            .await
            .unwrap();
        let db = Database { pool };
        assert_eq!(db.home_discovery().await.unwrap().track_count, 0);
        sqlx::query(
            "INSERT INTO files(id,path,file_name,status,missing_since,size) VALUES
            (1,'/music/one.flac','one.flac','ok',NULL,100),(2,'/musical/two.flac','two.flac','ok',NULL,200),
            (3,'/music/gone.flac','gone.flac','ok',1,300),(4,'/music/pending.flac','pending.flac','pending',NULL,400)",
        )
        .execute(&db.pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO metadata_texts(file_id,key,value) VALUES (1,'title','First song'),(1,'artist','An artist')")
            .execute(&db.pool).await.unwrap();
        sqlx::query("INSERT INTO listening_history(session_id,file_id,played_at) VALUES ('a',1,10),('b',1,20),('c',3,30)")
            .execute(&db.pool).await.unwrap();
        sqlx::query("INSERT INTO media_info(file_id,loved,plays) VALUES (1,1,2),(3,1,3)")
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO counted_plays(session_id,file_id,played_at) VALUES ('new',1,unixepoch()),('old',2,1),('missing',3,unixepoch())")
            .execute(&db.pool).await.unwrap();
        sqlx::query("INSERT INTO import_roots(path) VALUES ('/music/')")
            .execute(&db.pool)
            .await
            .unwrap();
        let home = db.home_discovery().await.unwrap();
        assert_eq!(home.track_count, 2);
        assert_eq!(home.recent.len(), 1);
        assert_eq!(home.recent[0].title, "First song");
        assert_eq!(home.heavy_rotation.len(), 1);
        assert_eq!(home.heavy_rotation[0].id, 1);
        assert_eq!(home.folders[0].track_count, 1);
        assert_eq!(home.folders[0].total_size, 100);
        assert_eq!(home.rediscover.len(), 2);
        assert_eq!(home.rediscover[0].title, "two.flac");
        assert!(db.files_by_ids(&[]).await.unwrap().is_empty());
        let files = db.files_by_ids(&[1, 2, 999]).await.unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "/music/one.flac");
        let songs = db.songs_by_ids(&[1, 2, 999]).await.unwrap();
        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].title, "First song");
        assert_eq!(songs[1].artist, "Unknown artist");
    }
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct TrackDisplay {
    pub title: String,
    pub artist: String,
    pub album: String,
}
impl Database {
    pub async fn track_display(&self, id: i64) -> Result<Option<TrackDisplay>, sqlx::Error> {
        sqlx::query_as("SELECT
            COALESCE((SELECT NULLIF(TRIM(value),'') FROM metadata_texts WHERE file_id=f.id AND key='title' ORDER BY ord LIMIT 1), f.file_name) AS title,
            COALESCE((SELECT NULLIF(TRIM(value),'') FROM metadata_texts WHERE file_id=f.id AND key='artist' ORDER BY ord LIMIT 1), 'Unknown artist') AS artist,
            COALESCE((SELECT NULLIF(TRIM(value),'') FROM metadata_texts WHERE file_id=f.id AND key='album' ORDER BY ord LIMIT 1), '') AS album
            FROM files f WHERE f.id=?")
            .bind(id).fetch_optional(&self.pool).await
    }
}
