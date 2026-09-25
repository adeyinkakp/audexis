use crate::commands::library::get_files::FilesResponse;
use crate::database::types::{DatabaseMediaFile, DatabaseMediaMetadata};
use crate::AppState;
use sqlx::{QueryBuilder, Sqlite};

#[tauri::command]
pub async fn get_media_files(
    state: tauri::State<'_, AppState>,
    ids: Vec<i64>,
) -> Result<FilesResponse, String> {
    if ids.len() > 200 {
        return Err("Request at most 200 files at a time".into());
    }
    if ids.is_empty() {
        return Ok(FilesResponse {
            files: vec![],
            metadata: vec![],
            next_cursor: None,
        });
    }

    let mut tx = state
        .db
        .pool
        .begin()
        .await
        .map_err(|error| error.to_string())?;
    let mut files = QueryBuilder::<Sqlite>::new("SELECT * FROM files WHERE id IN (");
    files.push_bind(ids[0]);
    for id in &ids[1..] {
        files.push(",").push_bind(id);
    }
    files.push(") ORDER BY id");
    let files = files
        .build_query_as::<DatabaseMediaFile>()
        .fetch_all(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    let mut metadata = QueryBuilder::<Sqlite>::new("SELECT file_id, key, value, ord FROM metadata_texts WHERE key IN ('title', 'album', 'artist', 'albumArtist', 'genre') AND file_id IN (");
    metadata.push_bind(ids[0]);
    for id in &ids[1..] {
        metadata.push(",").push_bind(id);
    }
    metadata.push(") ORDER BY file_id, ord");
    let metadata = metadata
        .build_query_as::<DatabaseMediaMetadata>()
        .fetch_all(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    tx.commit().await.map_err(|error| error.to_string())?;
    Ok(FilesResponse {
        files,
        metadata,
        next_cursor: None,
    })
}
