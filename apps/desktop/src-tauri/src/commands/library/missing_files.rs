use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::file_watcher::FileWatcher;
use crate::tag_manager::tag_backend::{DefaultBackend, TagBackend};
use crate::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct MissingFile {
    id: i64,
    path: String,
    file_name: String,
    missing_since: i64,
    title: String,
    artist: String,
}

#[derive(Clone, Serialize)]
struct LibraryChange {
    file_ids: Vec<i64>,
}

#[tauri::command]
pub async fn get_missing_files(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<MissingFile>, String> {
    sqlx::query_as::<_, MissingFile>(
        "SELECT f.id, f.path, f.file_name, f.missing_since,
                COALESCE((SELECT value FROM metadata_texts WHERE file_id=f.id AND key='title' ORDER BY ord LIMIT 1), '') AS title,
                COALESCE((SELECT value FROM metadata_texts WHERE file_id=f.id AND key='artist' ORDER BY ord LIMIT 1), '') AS artist
         FROM files f
         WHERE f.missing_since IS NOT NULL
         ORDER BY f.missing_since DESC, f.id",
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_missing_file(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    file_id: i64,
) -> Result<(), String> {
    let deleted = sqlx::query_scalar::<_, i64>(
        "DELETE FROM files WHERE id = ?1 AND missing_since IS NOT NULL RETURNING id",
    )
    .bind(file_id)
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|error| error.to_string())?;
    let Some(file_id) = deleted else {
        return Err("Missing file record was not found".into());
    };

    crate::commands::playback::shared::reconcile_queue_with_library(
        &app,
        &state,
        HashMap::new(),
        HashSet::from([file_id]),
    )
    .await?;
    app.emit(
        "library-changed",
        LibraryChange {
            file_ids: vec![file_id],
        },
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn relink_missing_file(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    file_id: i64,
) -> Result<Option<String>, String> {
    let selected = rfd::FileDialog::new()
        .set_title("Locate the audio file")
        .add_filter(
            "Audio",
            &["mp3", "flac", "ogg", "opus", "m4a", "mp4", "wav"],
        )
        .pick_file();
    let Some(path) = selected else {
        return Ok(None);
    };
    if !crate::file_watcher::is_audio_file(&path) {
        return Err("Select a supported audio file".into());
    }
    let metadata = std::fs::metadata(&path).map_err(|error| error.to_string())?;
    let path_string = path.to_string_lossy().to_string();
    let existing_id = sqlx::query_scalar::<_, i64>("SELECT id FROM files WHERE path = ?1")
        .bind(&path_string)
        .fetch_optional(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?;

    let mut tx = state
        .db
        .pool
        .begin()
        .await
        .map_err(|error| error.to_string())?;
    let is_missing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM files WHERE id = ?1 AND missing_since IS NOT NULL",
    )
    .bind(file_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| error.to_string())?
        > 0;
    if !is_missing {
        return Err("Missing file record was not found".into());
    }

    let merged_id = existing_id.filter(|id| *id != file_id);
    if let Some(other_id) = merged_id {
        sqlx::query(
            "INSERT INTO media_info (file_id, plays, loved)
             SELECT ?1, plays, loved FROM media_info WHERE file_id=?2
             ON CONFLICT(file_id) DO UPDATE SET
               plays = media_info.plays + excluded.plays,
               loved = media_info.loved OR excluded.loved",
        )
        .bind(file_id)
        .bind(other_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
        sqlx::query("UPDATE playlist_tracks SET file_id=?1 WHERE file_id=?2")
            .bind(file_id)
            .bind(other_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("UPDATE counted_plays SET file_id=?1 WHERE file_id=?2")
            .bind(file_id)
            .bind(other_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("UPDATE listening_history SET file_id=?1 WHERE file_id=?2")
            .bind(file_id)
            .bind(other_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query(
            "INSERT INTO listening_time_daily (file_id, source_playlist_id, day, heard_us)
             SELECT ?1, source_playlist_id, day, heard_us FROM listening_time_daily WHERE file_id=?2
             ON CONFLICT(file_id, source_playlist_id, day) DO UPDATE SET
               heard_us = listening_time_daily.heard_us + excluded.heard_us",
        )
        .bind(file_id)
        .bind(other_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM files WHERE id=?1")
            .bind(other_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
    }

    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");
    sqlx::query(
        "UPDATE files SET path=?1, file_name=?2, size=?3, modified_at=?4,
         status='pending', metadata_status='ok', missing_since=NULL WHERE id=?5",
    )
    .bind(&path_string)
    .bind(file_name)
    .bind(metadata.len() as i64)
    .bind(modified_at)
    .bind(file_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;
    tx.commit().await.map_err(|error| error.to_string())?;

    let read_path = PathBuf::from(&path_string);
    let parsed =
        tauri::async_runtime::spawn_blocking(move || DefaultBackend::new().read(&read_path))
            .await
            .map_err(|error| error.to_string())?
            .map_err(|error| format!("Could not read selected file: {error:?}"))?;
    FileWatcher::store_metadata(&state.db.pool, file_id, &parsed)
        .await
        .map_err(|error| format!("Could not rebuild metadata: {error:?}"))?;

    let mut missing_ids = HashSet::new();
    if let Some(other_id) = merged_id {
        missing_ids.insert(other_id);
    }
    crate::commands::playback::shared::reconcile_queue_with_library(
        &app,
        &state,
        HashMap::from([(file_id, path_string.clone())]),
        missing_ids,
    )
    .await?;
    let mut changed = vec![file_id];
    if let Some(other_id) = merged_id {
        changed.push(other_id);
    }
    app.emit("library-changed", LibraryChange { file_ids: changed })
        .map_err(|error| error.to_string())?;
    Ok(Some(path_string))
}
