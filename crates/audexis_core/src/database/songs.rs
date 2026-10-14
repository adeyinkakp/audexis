use super::Database;
pub use crate::config::SongColumn as SongSort;
use sqlx::{QueryBuilder, Sqlite};
use std::collections::HashMap;

const SONG_SOURCE: &str = "SELECT f.id,f.path,f.file_name,f.size,f.modified_at,
            COALESCE(strftime('%Y-%m-%d %H:%M',NULLIF(f.modified_at,0),'unixepoch','localtime'),'—') AS modified_label,
            COALESCE((SELECT NULLIF(TRIM(value),'') FROM metadata_texts WHERE file_id=f.id AND key='title' ORDER BY ord LIMIT 1),f.file_name) AS title,
            COALESCE((SELECT NULLIF(TRIM(value),'') FROM metadata_texts WHERE file_id=f.id AND key='artist' ORDER BY ord LIMIT 1),'Unknown artist') AS artist,
            COALESCE((SELECT NULLIF(TRIM(value),'') FROM metadata_texts WHERE file_id=f.id AND key='album' ORDER BY ord LIMIT 1),'') AS album,
            f.duration_ms,COALESCE(f.format,'') AS format,
            COALESCE((SELECT plays FROM media_info WHERE file_id=f.id),0) AS plays,
            COALESCE((SELECT loved FROM media_info WHERE file_id=f.id),0) AS loved
            FROM files f WHERE f.status='ok' AND f.missing_since IS NULL";

pub const SONG_PAGE_SIZE: usize = 100;
#[derive(Clone, Debug, PartialEq, sqlx::FromRow)]
pub struct SongRow {
    pub id: i64,
    pub path: String,
    pub file_name: String,
    pub size: i64,
    pub modified_at: i64,
    pub modified_label: String,
    pub plays: i64,
    pub loved: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: Option<i64>,
    pub format: String,
    #[sqlx(skip)]
    pub metadata: HashMap<String, String>,
}
impl SongRow {
    pub fn column_text(&self, column: &SongSort) -> String {
        match column {
            SongSort::Title => self.title.clone(),
            SongSort::Artist => self.artist.clone(),
            SongSort::Album => self.album.clone(),
            SongSort::Format => self.format.to_uppercase(),
            SongSort::Duration => self
                .duration_ms
                .filter(|v| *v >= 0)
                .map(|ms| format!("{}:{:02}", ms / 60000, (ms / 1000) % 60))
                .unwrap_or_else(|| "—".into()),
            SongSort::FileName => self.file_name.clone(),
            SongSort::Path => self.path.clone(),
            SongSort::Size => format!("{:.1} MB", self.size as f64 / 1_000_000.0),
            SongSort::Modified => self.modified_label.clone(),
            SongSort::Plays => self.plays.to_string(),
            SongSort::Loved => if self.loved { "♥" } else { "" }.into(),
            column => column
                .storage_key(&self.format)
                .and_then(|key| self.metadata.get(&key).cloned())
                .unwrap_or_default(),
        }
    }
}
pub struct SongPage {
    pub rows: Vec<SongRow>,
    pub total: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SongQuery {
    pub search: String,
    pub sort: SongSort,
    pub descending: bool,
}
#[derive(sqlx::FromRow)]
pub struct QueueSong {
    pub id: i64,
    pub path: String,
}
pub struct SongQueue {
    pub tracks: Vec<QueueSong>,
    pub index: usize,
}

fn filtered<'a>(select: &str, search: &str) -> QueryBuilder<'a, Sqlite> {
    let mut sql = QueryBuilder::new(format!(
        "{select} FROM ({SONG_SOURCE}) AS songs WHERE instr(lower(title || ' ' || artist || ' ' || album || ' ' || path),lower("
    ));
    sql.push_bind(search.trim().to_owned()).push(")) > 0");
    sql
}
fn order(sql: &mut QueryBuilder<'_, Sqlite>, sort: &SongSort, descending: bool) {
    sql.push(" ORDER BY ");
    let plain = match sort {
        SongSort::Title => {
            Some("NULLIF(TRIM(title, char(9) || char(10) || char(13) || ' '), '') COLLATE NOCASE")
        }
        SongSort::Artist => {
            Some("NULLIF(TRIM(artist, char(9) || char(10) || char(13) || ' '), '') COLLATE NOCASE")
        }
        SongSort::Album => {
            Some("NULLIF(TRIM(album, char(9) || char(10) || char(13) || ' '), '') COLLATE NOCASE")
        }
        SongSort::Duration => Some("CASE WHEN duration_ms >= 0 THEN duration_ms END"),
        SongSort::Format => {
            Some("NULLIF(TRIM(format, char(9) || char(10) || char(13) || ' '), '') COLLATE NOCASE")
        }
        SongSort::FileName => Some(
            "NULLIF(TRIM(file_name, char(9) || char(10) || char(13) || ' '), '') COLLATE NOCASE",
        ),
        SongSort::Path => {
            Some("NULLIF(TRIM(path, char(9) || char(10) || char(13) || ' '), '') COLLATE NOCASE")
        }
        SongSort::Size => Some("size"),
        SongSort::Modified => Some("NULLIF(modified_at, 0)"),
        SongSort::Plays => Some("plays"),
        SongSort::Loved => Some("loved"),
        _ => None,
    };
    if let Some(column) = plain {
        sql.push(column);
    } else {
        let numeric = matches!(sort, SongSort::Metadata(key) if matches!(key.as_str(), "year" | "trackNumber" | "discnumber" | "totaltracks" | "totaldiscs" | "beatsPerMinute" | "_rating" | "playCount" | "movementnumber" | "movementtotal"));
        if numeric {
            sql.push("CAST(");
        }
        sql.push("(SELECT group_concat(value, '; ') FROM (SELECT value FROM metadata_texts WHERE file_id=songs.id AND key=");
        if matches!(sort, SongSort::Custom { .. }) {
            sql.push("CASE songs.format WHEN 'FLAC' THEN ")
                .push_bind(sort.storage_key("FLAC").unwrap())
                .push(" WHEN 'Ogg' THEN ")
                .push_bind(sort.storage_key("Ogg").unwrap())
                .push(" WHEN 'Itunes' THEN ")
                .push_bind(sort.storage_key("Itunes").unwrap())
                .push(" ELSE ")
                .push_bind(sort.storage_key("").unwrap())
                .push(" END");
        } else {
            sql.push_bind(sort.storage_key("").unwrap());
        }
        sql.push(" AND NULLIF(TRIM(value, char(9) || char(10) || char(13) || ' '), '') IS NOT NULL ORDER BY ord))");
        if numeric {
            sql.push(" AS REAL)");
        } else {
            sql.push(" COLLATE NOCASE");
        }
    }
    sql.push(if descending { " DESC" } else { " ASC" })
        .push(" NULLS LAST,songs.id ASC");
}
impl Database {
    pub async fn songs_queue(
        &self,
        query: &SongQuery,
        selected_id: i64,
    ) -> Result<SongQueue, String> {
        let mut sql = filtered("SELECT id,path", &query.search);
        order(&mut sql, &query.sort, query.descending);
        let tracks: Vec<QueueSong> = sql
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        let index = tracks
            .iter()
            .position(|t| t.id == selected_id)
            .ok_or_else(|| {
                "This song no longer matches the library selection. Refresh and try again."
                    .to_string()
            })?;
        Ok(SongQueue { tracks, index })
    }
    pub async fn songs_page(
        &self,
        search: &str,
        sort: SongSort,
        descending: bool,
        offset: usize,
    ) -> Result<SongPage, sqlx::Error> {
        self.songs_page_columns(search, sort, descending, offset, &[])
            .await
    }
    pub async fn songs_page_columns(
        &self,
        search: &str,
        sort: SongSort,
        descending: bool,
        offset: usize,
        columns: &[SongSort],
    ) -> Result<SongPage, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let total = filtered("SELECT COUNT(*)", search)
            .build_query_scalar()
            .fetch_one(&mut *tx)
            .await?;
        let mut sql = filtered("SELECT *", search);
        order(&mut sql, &sort, descending);
        sql.push(" LIMIT ")
            .push_bind(SONG_PAGE_SIZE as i64)
            .push(" OFFSET ")
            .push_bind(offset.min(i64::MAX as usize) as i64);
        let mut rows: Vec<SongRow> = sql.build_query_as().fetch_all(&mut *tx).await?;
        let keys: std::collections::HashSet<String> = rows
            .iter()
            .flat_map(|row| {
                columns
                    .iter()
                    .filter_map(|col| col.storage_key(&row.format))
            })
            .collect();
        if !rows.is_empty() && !keys.is_empty() {
            let mut sql = QueryBuilder::<Sqlite>::new(
                "SELECT file_id,key,value FROM metadata_texts WHERE file_id IN (",
            );
            {
                let mut ids = sql.separated(",");
                for row in &rows {
                    ids.push_bind(row.id);
                }
            }
            sql.push(") AND key IN (");
            {
                let mut values = sql.separated(",");
                for key in keys {
                    values.push_bind(key);
                }
            }
            sql.push(") ORDER BY file_id,key,ord");
            let values: Vec<(i64, String, String)> =
                sql.build_query_as().fetch_all(&mut *tx).await?;
            let indices: HashMap<i64, usize> = rows
                .iter()
                .enumerate()
                .map(|(index, r)| (r.id, index))
                .collect();
            for (id, key, value) in values {
                let cell = rows[*indices.get(&id).unwrap()]
                    .metadata
                    .entry(key)
                    .or_default();
                if !cell.is_empty() {
                    cell.push_str("; ");
                }
                cell.push_str(&value);
            }
        }
        tx.commit().await?;
        Ok(SongPage { rows, total })
    }
}

#[cfg(test)]
mod tests;
