use super::{Database, discovery::DiscoveryTrack, types::DatabaseMediaFile};
use sqlx::{QueryBuilder, Sqlite};

impl Database {
    pub async fn files_by_ids(&self, ids: &[i64]) -> Result<Vec<DatabaseMediaFile>, sqlx::Error> {
        let mut files = Vec::new();
        for chunk in ids.chunks(500) {
            let mut query = QueryBuilder::<Sqlite>::new(
                "SELECT id,path,file_name,last_validated,status,duration_ms,format,modified_at,size,missing_since FROM files WHERE id IN (",
            );
            let mut values = query.separated(",");
            for id in chunk {
                values.push_bind(id);
            }
            query.push(") ORDER BY id");
            files.extend(query.build_query_as().fetch_all(&self.pool).await?);
        }
        Ok(files)
    }

    pub async fn songs_by_ids(&self, ids: &[i64]) -> Result<Vec<DiscoveryTrack>, sqlx::Error> {
        let mut songs = Vec::new();
        for chunk in ids.chunks(500) {
            let mut query = QueryBuilder::<Sqlite>::new("SELECT f.id,f.path,
                COALESCE((SELECT NULLIF(TRIM(value),'') FROM metadata_texts WHERE file_id=f.id AND key='title' ORDER BY ord LIMIT 1),f.file_name) AS title,
                COALESCE((SELECT NULLIF(TRIM(value),'') FROM metadata_texts WHERE file_id=f.id AND key='artist' ORDER BY ord LIMIT 1),'Unknown artist') AS artist,
                COALESCE((SELECT NULLIF(TRIM(value),'') FROM metadata_texts WHERE file_id=f.id AND key='album' ORDER BY ord LIMIT 1),'') AS album,
                f.duration_ms FROM files f WHERE f.id IN (");
            let mut values = query.separated(",");
            for id in chunk {
                values.push_bind(id);
            }
            query.push(") ORDER BY f.id");
            songs.extend(query.build_query_as().fetch_all(&self.pool).await?);
        }
        Ok(songs)
    }
}
