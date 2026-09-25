use crate::AppState;
use sqlx::{QueryBuilder, Sqlite};

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionKind {
    Albums,
    Artists,
    Playlists,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct Collection {
    name: String,
    artist: String,
    file_id: Option<i64>,
    playlist_id: Option<i64>,
    track_count: i64,
}
#[derive(serde::Serialize)]
pub struct CollectionPage {
    items: Vec<Collection>,
    next_cursor: Option<i64>,
}

#[tauri::command]
pub async fn browse_library(
    state: tauri::State<'_, AppState>,
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
        CollectionKind::Albums => "SELECT a.value AS name, COALESCE((SELECT m.value FROM metadata_texts m WHERE m.file_id=f.id AND m.key IN ('albumArtist','artist') AND TRIM(m.value) <> '' ORDER BY CASE m.key WHEN 'albumArtist' THEN 0 ELSE 1 END, m.ord LIMIT 1), '') AS artist, f.id AS file_id, NULL AS playlist_id FROM files f JOIN metadata_texts a ON a.file_id=f.id AND a.key='album' WHERE TRIM(a.value) <> '' UNION ALL SELECT '' AS name, '' AS artist, f.id AS file_id, NULL AS playlist_id FROM files f WHERE NOT EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key='album' AND TRIM(m.value) <> '')",
        CollectionKind::Artists => "SELECT m.value AS name, '' AS artist, f.id AS file_id, NULL AS playlist_id FROM files f JOIN metadata_texts m ON m.file_id=f.id WHERE m.key IN ('artist','albumArtist') AND TRIM(m.value) <> '' UNION ALL SELECT '' AS name, '' AS artist, f.id AS file_id, NULL AS playlist_id FROM files f WHERE NOT EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key IN ('artist','albumArtist') AND TRIM(m.value) <> '')",
        CollectionKind::Playlists => "SELECT p.name, '' AS artist, pt.file_id, p.id AS playlist_id FROM playlists p LEFT JOIN playlist_tracks pt ON pt.playlist_id=p.id",
    };
    let mut sql = QueryBuilder::<Sqlite>::new("WITH entries AS (");
    sql.push(source).push(") SELECT name, artist, MIN(file_id) AS file_id, playlist_id, COUNT(DISTINCT file_id) AS track_count FROM entries WHERE 1=1");
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
        .fetch_all(&state.db.pool)
        .await
        .map_err(|e| e.to_string())?;
    let next_cursor = (items.len() > 24).then_some(offset + 24);
    items.truncate(24);
    Ok(CollectionPage { items, next_cursor })
}
