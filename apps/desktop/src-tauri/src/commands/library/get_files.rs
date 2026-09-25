use tauri::command;

use crate::database::types::{DatabaseMediaFile, DatabaseMediaMetadata};

use crate::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct FilesResponse {
    pub files: Vec<DatabaseMediaFile>,
    pub metadata: Vec<DatabaseMediaMetadata>,
    pub next_cursor: Option<i64>,
}

#[command]
pub async fn get_media_page(
    state: tauri::State<'_, AppState>,
    cursor: Option<i64>,
    limit: Option<i64>,
) -> Result<FilesResponse, ()> {
    let db = state.db.clone();
    let cursor = cursor.unwrap_or(0);
    let limit = limit.unwrap_or(100).clamp(1, 200);
    let mut tx = db.pool.begin().await.map_err(|_| ())?;
    let files = sqlx::query_as::<_, DatabaseMediaFile>(
        "SELECT * FROM files
         WHERE id > ?1
         ORDER BY id
         LIMIT ?2",
    )
    .bind(cursor)
    .bind(limit)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| ())?;
    let next_cursor = if files.len() == limit as usize {
        files.last().map(|file| file.id)
    } else {
        None
    };
    let metadata = sqlx::query_as::<_, DatabaseMediaMetadata>(
        "SELECT file_id, key, value, ord
         FROM metadata_texts
         WHERE key IN ('title', 'album', 'artist', 'genre')
           AND file_id IN (
               SELECT id FROM files
               WHERE id > ?1
               ORDER BY id
               LIMIT ?2
           )
         ORDER BY file_id, ord",
    )
    .bind(cursor)
    .bind(limit)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| ())?;

    tx.commit().await.map_err(|_| ())?;
    Ok(FilesResponse {
        files,
        metadata,
        next_cursor,
    })
}
