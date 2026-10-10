use crate::tag_manager::{
    tag_backend::{DefaultBackend, TagBackend},
    utils::{FrameKey, TagValue},
};
use crate::AppState;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsInfo {
    file_id: i64,
    plain: Vec<String>,
    synced: Vec<String>,
    supported: bool,
}

#[tauri::command]
pub async fn get_lyrics(
    state: tauri::State<'_, AppState>,
    file_ids: Vec<i64>,
) -> Result<Vec<LyricsInfo>, String> {
    if file_ids.len() > 200 {
        return Err("Select at most 200 tracks".into());
    }
    let mut paths = Vec::new();
    for id in file_ids {
        let path = sqlx::query_scalar::<_, String>("SELECT path FROM files WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db.pool)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Track {id} no longer exists"))?;
        paths.push((id, path));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let backend = DefaultBackend::new();
        paths
            .into_iter()
            .map(|(file_id, path)| {
                let metadata = backend
                    .read(Path::new(&path))
                    .map_err(|e| super::update_metadata::error_message(&e))?;
                let texts = |key| {
                    metadata
                        .tags
                        .get(&key)
                        .into_iter()
                        .flatten()
                        .filter_map(|v| match v {
                            TagValue::Text(text) => Some(text.clone()),
                            _ => None,
                        })
                        .collect()
                };
                use crate::tag_manager::traits::Formats;
                Ok(LyricsInfo {
                    file_id,
                    plain: texts(FrameKey::UnsyncedLyrics),
                    synced: texts(FrameKey::SynchronizedLyrics),
                    supported: matches!(
                        metadata.tag_format,
                        Formats::Id3v22
                            | Formats::Id3v23
                            | Formats::Id3v24
                            | Formats::Flac
                            | Formats::Ogg
                            | Formats::Itunes
                    ),
                })
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())?
}
