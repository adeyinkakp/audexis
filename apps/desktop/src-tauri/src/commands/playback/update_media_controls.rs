use crate::audio_player::PartialMetadata;
use std::collections::HashMap;
use tauri::{command, Manager};

use crate::AppState;

#[command]
pub async fn update_media_controls(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    file_id: i64,
) -> Result<(), String> {
    let file = sqlx::query_as::<_, (String, Option<i64>)>(
        "SELECT file_name, duration_ms FROM files WHERE id = ?1",
    )
    .bind(file_id)
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "Song is no longer in the library".to_string())?;

    let values = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM metadata_texts
         WHERE file_id = ?1 AND key IN ('title', 'album', 'artist')
         ORDER BY ord",
    )
    .bind(file_id)
    .fetch_all(&state.db.pool)
    .await
    .map_err(|error| error.to_string())?
    .into_iter()
    .fold(HashMap::new(), |mut values, (key, value)| {
        values.entry(key).or_insert(value);
        values
    });

    let artwork = sqlx::query_as::<_, (Vec<u8>, String)>(
        "SELECT data, mime_type FROM metadata_pictures
         WHERE file_id = ?1 ORDER BY id LIMIT 1",
    )
    .bind(file_id)
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|error| error.to_string())?;

    let cover_url = if let Some((data, mime)) = artwork {
        let extension = match mime.as_str() {
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            _ => "jpg",
        };
        let directory = app
            .path()
            .app_cache_dir()
            .map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let path = directory.join(format!("media-controls-cover-{file_id}.{extension}"));
        std::fs::write(&path, data).map_err(|error| error.to_string())?;
        tauri::Url::from_file_path(path)
            .ok()
            .map(|url| url.to_string())
    } else {
        None
    };

    let non_empty =
        |value: Option<&String>| value.filter(|value| !value.trim().is_empty()).cloned();
    let metadata = PartialMetadata {
        title: non_empty(values.get("title")).or(Some(file.0)),
        album: non_empty(values.get("album")),
        artist: non_empty(values.get("artist")),
        cover_url,
        duration: file.1.and_then(|duration| u64::try_from(duration).ok()),
    };
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    player.update_media_controls(metadata);
    Ok(())
}
