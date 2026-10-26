use super::{Database, songs::SongRow};
use crate::database::search::refresh_pending;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionKind {
    Albums,
    Artists,
    Playlists,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, sqlx::FromRow)]
pub struct Collection {
    pub name: String,
    pub artist: String,
    pub file_id: Option<i64>,
    pub playlist_id: Option<i64>,
    pub track_count: i64,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct CollectionPage {
    pub items: Vec<Collection>,
    pub next_cursor: Option<i64>,
}

pub async fn browse_library_with_pool(
    pool: &SqlitePool,
    kind: CollectionKind,
    query: String,
    artist: Option<String>,
    offset: i64,
) -> Result<CollectionPage, String> {
    if query.len() > 500
        || query.split_whitespace().count() > 12
        || offset < 0
        || artist.as_ref().is_some_and(|v| v.len() > 500)
    {
        return Err("Invalid search range or search text".into());
    }

    let source = match kind {
        CollectionKind::Albums => {
            "SELECT a.value AS name, COALESCE((SELECT m.value FROM metadata_texts m WHERE m.file_id=f.id AND m.key IN ('albumArtist','artist') AND TRIM(m.value) <> '' ORDER BY CASE m.key WHEN 'albumArtist' THEN 0 ELSE 1 END, m.ord LIMIT 1), '') AS artist, f.id AS file_id, NULL AS playlist_id FROM files f JOIN metadata_texts a ON a.file_id=f.id AND a.key='album' WHERE f.missing_since IS NULL AND TRIM(a.value) <> '' UNION ALL SELECT '' AS name, '' AS artist, f.id AS file_id, NULL AS playlist_id FROM files f WHERE f.missing_since IS NULL AND NOT EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key='album' AND TRIM(m.value) <> '')"
        }
        CollectionKind::Artists => {
            "SELECT m.value AS name, '' AS artist, f.id AS file_id, NULL AS playlist_id FROM files f JOIN metadata_texts m ON m.file_id=f.id WHERE f.missing_since IS NULL AND m.key IN ('artist','albumArtist') AND TRIM(m.value) <> '' UNION ALL SELECT '' AS name, '' AS artist, f.id AS file_id, NULL AS playlist_id FROM files f WHERE f.missing_since IS NULL AND NOT EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key IN ('artist','albumArtist') AND TRIM(m.value) <> '')"
        }
        CollectionKind::Playlists => {
            "SELECT p.name, '' AS artist, CASE WHEN f.id IS NOT NULL THEN pt.file_id END AS file_id, p.id AS playlist_id FROM library_playlists p LEFT JOIN playlist_tracks pt ON pt.playlist_id=p.id LEFT JOIN files f ON f.id=pt.file_id AND f.missing_since IS NULL"
        }
    };
    let indexed_kind = match kind {
        CollectionKind::Albums => Some("album"),
        CollectionKind::Artists => Some("artist"),
        CollectionKind::Playlists => None,
    }
    .filter(|_| !query.trim().is_empty());
    let mut sql = QueryBuilder::<Sqlite>::new("WITH entries AS (");
    if let Some(kind) = indexed_kind {
        refresh_pending(pool).await.map_err(|e| e.to_string())?;
        sql.push("SELECT d.name, d.artist, d.file_id, NULL AS playlist_id \
            FROM collection_search JOIN collection_search_documents d ON d.id=collection_search.rowid \
            JOIN files f ON f.id=d.file_id WHERE f.missing_since IS NULL AND d.kind=")
            .push_bind(kind)
            .push(" AND collection_search MATCH ").push_bind(prefix_terms(&query));

        if query
            .split_whitespace()
            .all(|term| "uncategorized".starts_with(&term.to_lowercase()))
        {
            sql.push(" UNION ALL SELECT d.name,d.artist,d.file_id,NULL FROM collection_search_documents d \
                JOIN files f ON f.id=d.file_id WHERE f.missing_since IS NULL AND d.name='' AND d.kind=")
                .push_bind(kind);
        }
    } else {
        sql.push(source);
    }
    sql.push(") SELECT name, artist, MIN(file_id) AS file_id, playlist_id, ");
    sql.push(if matches!(kind, CollectionKind::Playlists) {
        "COUNT(file_id)"
    } else {
        "COUNT(DISTINCT file_id)"
    });
    sql.push(" AS track_count FROM entries WHERE 1=1");

    if indexed_kind.is_none() {
        for term in query.split_whitespace() {
            let escaped = format!(
                "%{}%",
                term.replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            );
            sql.push(" AND (CASE WHEN name = '' THEN 'Uncategorized' ELSE name END LIKE ")
                .push_bind(escaped.clone())
                .push(" ESCAPE '\\' OR artist LIKE ")
                .push_bind(escaped)
                .push(" ESCAPE '\\')");
        }
    }
    if let Some(artist) = artist {
        if artist.is_empty() {
            sql.push(" AND NOT EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=entries.file_id AND m.key IN ('artist','albumArtist') AND TRIM(m.value) <> '')");
        } else {
            sql.push(" AND EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=entries.file_id AND m.key IN ('artist','albumArtist') AND m.value = ").push_bind(artist).push(" COLLATE NOCASE)");
        }
    }
    sql.push(" GROUP BY name COLLATE NOCASE, artist COLLATE NOCASE, playlist_id ORDER BY CASE WHEN name = ").push_bind(query.trim().to_owned()).push(" COLLATE NOCASE THEN 0 ELSE 1 END, name COLLATE NOCASE, artist COLLATE NOCASE, playlist_id LIMIT 25 OFFSET ").push_bind(offset);
    let mut items = sql
        .build_query_as::<Collection>()
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let next_cursor = (items.len() > 24).then_some(offset + 24);
    items.truncate(24);
    Ok(CollectionPage { items, next_cursor })
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackFilter {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub playlist: Option<i64>,
    pub favorites: bool,
    pub search: String,
}
impl Database {
    pub async fn collection_tracks(
        &self,
        filter: TrackFilter,
    ) -> Result<Vec<SongRow>, sqlx::Error> {
        let mut q = QueryBuilder::<Sqlite>::new(format!(
            "SELECT songs.* FROM ({}) songs WHERE 1=1",
            super::songs::SONG_SOURCE
        ));
        if filter.favorites {
            q.push(" AND loved=1");
        }
        if let Some(id) = filter.playlist {
            q = QueryBuilder::new(format!(
                "SELECT songs.*,pt.ord AS playlist_ord FROM ({}) songs JOIN playlist_tracks pt ON pt.file_id=songs.id WHERE pt.playlist_id=",
                super::songs::SONG_SOURCE
            ));
            q.push_bind(id)
                .push(" AND EXISTS(SELECT 1 FROM library_playlists p WHERE p.id=pt.playlist_id)");
        }
        if let Some(artist) = filter.artist {
            if artist.is_empty() {
                q.push(" AND NOT EXISTS(SELECT 1 FROM metadata_texts m WHERE m.file_id=songs.id AND m.key IN ('artist','albumArtist') AND TRIM(m.value)<>'')");
            } else {
                q.push(" AND EXISTS(SELECT 1 FROM metadata_texts m WHERE m.file_id=songs.id AND m.key IN ('artist','albumArtist') AND m.value=").push_bind(artist).push(" COLLATE NOCASE)");
            }
        }
        if let Some(album) = filter.album {
            q.push(" AND album=")
                .push_bind(album)
                .push(" COLLATE NOCASE");
        }
        for term in filter.search.split_whitespace() {
            q.push(" AND instr(lower(title || ' ' || artist || ' ' || album), lower(")
                .push_bind(term.to_owned())
                .push("))>0");
        }
        q.push(if filter.playlist.is_some() {
            " ORDER BY pt.ord"
        } else {
            " ORDER BY title COLLATE NOCASE, id"
        });
        let mut rows: Vec<SongRow> = q.build_query_as().fetch_all(&self.pool).await?;
        self.validate_song_downloads(&mut rows).await?;
        Ok(rows)
    }
    pub async fn create_collection_playlist(&self, name: &str) -> Result<i64, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 200 {
            return Err("Enter a playlist name between 1 and 200 characters.".into());
        }
        sqlx::query("INSERT INTO playlists(name) VALUES (?)")
            .bind(name)
            .execute(&self.pool)
            .await
            .map(|r| r.last_insert_rowid())
            .map_err(|e| e.to_string())
    }
    pub async fn set_favorite(&self, id: i64, loved: bool) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO media_info(file_id,loved) VALUES (?,?) ON CONFLICT(file_id) DO UPDATE SET loved=excluded.loved").bind(id).bind(loved).execute(&self.pool).await?;
        Ok(())
    }
}

fn prefix_terms(value: &str) -> String {
    value
        .split_whitespace()
        .map(|term| format!("\"{}\"*", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

impl Database {
    pub async fn rename_collection_playlist(&self, id: i64, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 200 {
            return Err("Enter a playlist name between 1 and 200 characters.".into());
        }
        let result = sqlx::query("UPDATE playlists SET name=? WHERE id=?")
            .bind(name)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        if result.rows_affected() == 0 {
            return Err("Playlist no longer exists.".into());
        }
        Ok(())
    }
    pub async fn add_collection_track(&self, playlist: i64, file: i64) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO playlist_tracks(playlist_id,file_id,ord) SELECT ?,?,COALESCE(MAX(ord),-1)+1 FROM playlist_tracks WHERE playlist_id=?").bind(playlist).bind(file).bind(playlist).execute(&self.pool).await?;
        Ok(())
    }
    pub async fn remove_collection_track(
        &self,
        playlist: i64,
        file: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id=? AND ord=(SELECT MIN(ord) FROM playlist_tracks WHERE playlist_id=? AND file_id=?)").bind(playlist).bind(playlist).bind(file).execute(&self.pool).await?;
        Ok(())
    }
    pub async fn delete_collection_playlist(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM playlists WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
