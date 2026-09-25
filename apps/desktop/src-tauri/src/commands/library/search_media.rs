use super::get_files::FilesResponse;
use crate::{
    database::types::{DatabaseMediaFile, DatabaseMediaMetadata},
    AppState,
};
use sqlx::{QueryBuilder, Sqlite};

#[derive(Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchInput {
    query: String,
    favorites_only: bool,
    exact_album: Option<String>,
    exact_artist: Option<String>,
    title: String,
    artist: String,
    album: String,
    file_name: String,
    genre: String,
    format: String,
    folder: String,
    min_duration_ms: Option<i64>,
    max_duration_ms: Option<i64>,
    offset: i64,
}

fn pattern(value: &str) -> String {
    format!(
        "%{}%",
        value
            .trim()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    )
}
fn contains(query: &mut QueryBuilder<'_, Sqlite>, expression: &str, value: &str) {
    query
        .push(expression)
        .push(" LIKE ")
        .push_bind(pattern(value))
        .push(" ESCAPE '\\'");
}
fn tag_match(query: &mut QueryBuilder<'_, Sqlite>, key: Option<&str>, value: &str) {
    query.push("EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id = f.id AND ");
    if let Some(key) = key {
        query
            .push("m.key = ")
            .push_bind(key.to_owned())
            .push(" AND ");
    }
    contains(query, "m.value", value);
    query.push(")");
}

#[tauri::command]
pub async fn search_media(
    state: tauri::State<'_, AppState>,
    input: SearchInput,
) -> Result<FilesResponse, String> {
    let terms: Vec<_> = input.query.split_whitespace().collect();
    if terms.len() > 12
        || [
            &input.query,
            &input.title,
            &input.artist,
            &input.album,
            &input.file_name,
            &input.genre,
            &input.format,
            &input.folder,
        ]
        .iter()
        .any(|value| value.len() > 500)
    {
        return Err("Use up to 12 search words and 500 characters per field".into());
    }
    if input.offset < 0
        || input.min_duration_ms.is_some_and(|value| value < 0)
        || input.max_duration_ms.is_some_and(|value| value < 0)
        || matches!((input.min_duration_ms, input.max_duration_ms), (Some(min), Some(max)) if min > max)
    {
        return Err("Check the duration range".into());
    }
    if input.exact_album.as_ref().is_some_and(|v| v.len() > 500)
        || input.exact_artist.as_ref().is_some_and(|v| v.len() > 500)
    {
        return Err("Collection name is too long".into());
    }
    let mut query = QueryBuilder::<Sqlite>::new("SELECT f.* FROM files f WHERE 1=1");
    if input.favorites_only {
        query.push(
            " AND EXISTS (SELECT 1 FROM media_info i WHERE i.file_id = f.id AND i.loved = 1)",
        );
    }
    if let Some(album) = &input.exact_album {
        if album.is_empty() {
            query.push(" AND NOT EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key='album' AND TRIM(m.value) <> '')");
        } else {
            query.push(" AND EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key='album' AND m.value = ").push_bind(album.clone()).push(" COLLATE METADATA_NOCASE)");
        }
    }
    if let Some(artist) = &input.exact_artist {
        if artist.is_empty() {
            query.push(" AND NOT EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key IN ('artist','albumArtist') AND TRIM(m.value) <> '')");
        } else {
            query.push(" AND EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key IN ('artist','albumArtist') AND m.value = ").push_bind(artist.clone()).push(" COLLATE METADATA_NOCASE)");
        }
    }
    for term in &terms {
        query.push(" AND (");
        contains(&mut query, "f.file_name", term);
        query.push(" OR ");
        contains(&mut query, "f.path", term);
        query.push(" OR ");
        contains(&mut query, "f.format", term);
        query.push(" OR ");
        tag_match(&mut query, None, term);
        query.push(")");
    }
    for (key, value) in [
        ("title", &input.title),
        ("artist", &input.artist),
        ("album", &input.album),
        ("genre", &input.genre),
    ] {
        if !value.trim().is_empty() {
            query.push(" AND ");
            tag_match(&mut query, Some(key), value);
        }
    }
    for (expression, value) in [
        ("f.file_name", &input.file_name),
        ("f.format", &input.format),
        ("f.path", &input.folder),
    ] {
        if !value.trim().is_empty() {
            query.push(" AND ");
            contains(&mut query, expression, value);
        }
    }
    if let Some(min) = input.min_duration_ms {
        query.push(" AND f.duration_ms >= ").push_bind(min);
    }
    if let Some(max) = input.max_duration_ms {
        query.push(" AND f.duration_ms <= ").push_bind(max);
    }
    query.push(" ORDER BY (");

    query.push_bind(0_i64);

    for term in &terms {
        for (key, weight) in [("title", 100), ("artist", 90), ("album", 70)] {
            query.push(" + CASE WHEN EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id = f.id AND m.key = ")
                .push_bind(key.to_owned()).push(" AND m.value = ").push_bind(term.to_string()).push(" COLLATE NOCASE) THEN ").push_bind(weight * 3)
                .push(" WHEN ");
            tag_match(&mut query, Some(key), term);
            query.push(" THEN ").push_bind(weight).push(" ELSE 0 END");
        }
        query
            .push(" + CASE WHEN f.file_name = ")
            .push_bind(term.to_string())
            .push(" COLLATE NOCASE THEN 240 WHEN ");
        contains(&mut query, "f.file_name", term);
        query.push(" THEN 80 ELSE 0 END");
    }
    query
        .push(") DESC, f.file_name COLLATE NOCASE, f.id LIMIT 51 OFFSET ")
        .push_bind(input.offset);
    let mut tx = state
        .db
        .pool
        .begin()
        .await
        .map_err(|error| error.to_string())?;
    let mut files = query
        .build_query_as::<DatabaseMediaFile>()
        .fetch_all(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    let next_cursor = (files.len() > 50).then_some(input.offset + 50);
    files.truncate(50);
    let mut metadata = Vec::new();
    if !files.is_empty() {
        let mut tags = QueryBuilder::<Sqlite>::new("SELECT file_id, key, value, ord FROM metadata_texts WHERE key IN ('title', 'artist', 'album', 'albumArtist', 'genre') AND file_id IN (");
        let mut separated = tags.separated(",");
        for file in &files {
            separated.push_bind(file.id);
        }
        tags.push(") ORDER BY file_id, ord");
        metadata = tags
            .build_query_as::<DatabaseMediaMetadata>()
            .fetch_all(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
    }
    tx.commit().await.map_err(|error| error.to_string())?;
    Ok(FilesResponse {
        files,
        metadata,
        next_cursor,
    })
}
