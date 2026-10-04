use std::collections::HashMap;

use serde::Deserialize;
use sqlx::{QueryBuilder, Sqlite};
use tauri::{AppHandle, Emitter};

use crate::tag_manager::tag_backend::{DefaultBackend, TagBackend};
use crate::tag_manager::utils::{Changes, FrameKey, SerializableTagValue, TagChange, TagValue};
use crate::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMetadataInput {
    file_ids: Vec<i64>,
    changes: HashMap<FrameKey, TagChange>,
}

#[derive(Clone, serde::Serialize)]
struct LibraryChange<'a> {
    file_ids: &'a [i64],
}

struct StoredPicture {
    data: Vec<u8>,
    mime: String,
    picture_type: i64,
    description: String,
}

#[tauri::command]
pub async fn update_metadata(
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
    input: UpdateMetadataInput,
) -> Result<(), String> {
    if input.file_ids.is_empty() || input.changes.is_empty() {
        return Ok(());
    }
    if input.file_ids.len() > 200 {
        return Err("Update at most 200 files at a time".into());
    }
    for (key, change) in &input.changes {
        if let TagChange::Replace(values) = change {
            let valid = if *key == FrameKey::AttachedPicture {
                values
                    .iter()
                    .all(|value| matches!(value, SerializableTagValue::Picture { .. }))
            } else {
                values
                    .iter()
                    .all(|value| matches!(value, SerializableTagValue::Text(_)))
            };
            if !valid {
                return Err("Metadata values do not match their field type".into());
            }
        }
    }

    let mut paths_query = QueryBuilder::<Sqlite>::new("SELECT id, path FROM files WHERE id IN (");
    paths_query.push_bind(input.file_ids[0]);
    for id in &input.file_ids[1..] {
        paths_query.push(",").push_bind(id);
    }
    paths_query.push(") ORDER BY id");
    let rows = paths_query
        .build_query_as::<(i64, String)>()
        .fetch_all(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?;

    if rows.len() != input.file_ids.len() {
        return Err("One or more selected files no longer exist in the library".into());
    }

    let paths = rows.iter().map(|(_, path)| path.clone()).collect();
    let write_changes = Changes {
        paths,
        tags: input.changes.clone(),
    };
    let errors = tauri::async_runtime::spawn_blocking(move || {
        DefaultBackend::new().write_changes(&write_changes)
    })
    .await
    .map_err(|error| error.to_string())?;

    if !errors.is_empty() {
        let message = errors
            .into_iter()
            .map(|error| format!("{error:?}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(message);
    }

    let artwork_by_file = if input.changes.contains_key(&FrameKey::AttachedPicture) {
        let files = rows.clone();
        Some(
            tauri::async_runtime::spawn_blocking(move || {
                let backend = DefaultBackend::new();
                files
                    .into_iter()
                    .map(|(file_id, path)| {
                        let metadata = backend
                            .read(std::path::Path::new(&path))
                            .map_err(|error| format!("Could not reread artwork: {error:?}"))?;
                        let pictures = metadata
                            .tags
                            .get(&FrameKey::AttachedPicture)
                            .into_iter()
                            .flatten()
                            .filter_map(|value| match value {
                                TagValue::Picture {
                                    mime,
                                    data,
                                    picture_type,
                                    description,
                                } => Some(StoredPicture {
                                    data: data.clone(),
                                    mime: mime.clone(),
                                    picture_type: i64::from(picture_type.unwrap_or(3)),
                                    description: description.clone().unwrap_or_default(),
                                }),
                                _ => None,
                            })
                            .collect::<Vec<_>>();
                        Ok((file_id, pictures))
                    })
                    .collect::<Result<HashMap<_, _>, String>>()
            })
            .await
            .map_err(|error| error.to_string())??,
        )
    } else {
        None
    };

    let mut tx = state
        .db
        .pool
        .begin()
        .await
        .map_err(|error| error.to_string())?;
    for (key, change) in &input.changes {
        let key = key.to_string();
        for file_id in &input.file_ids {
            if key == FrameKey::AttachedPicture.to_string() {
                sqlx::query("DELETE FROM metadata_pictures WHERE file_id = ?1")
                    .bind(file_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|error| error.to_string())?;

                if let Some(pictures) = artwork_by_file
                    .as_ref()
                    .and_then(|pictures| pictures.get(file_id))
                {
                    for picture in pictures {
                        sqlx::query(
                            "INSERT INTO metadata_pictures
                             (file_id, data, mime_type, picture_type, description)
                             VALUES (?1, ?2, ?3, ?4, ?5)",
                        )
                        .bind(file_id)
                        .bind(&picture.data)
                        .bind(&picture.mime)
                        .bind(picture.picture_type)
                        .bind(&picture.description)
                        .execute(&mut *tx)
                        .await
                        .map_err(|error| error.to_string())?;
                    }
                }
                continue;
            }

            sqlx::query("DELETE FROM metadata_texts WHERE file_id = ?1 AND key = ?2")
                .bind(file_id)
                .bind(&key)
                .execute(&mut *tx)
                .await
                .map_err(|error| error.to_string())?;

            if let TagChange::Replace(values) = change {
                for (ord, value) in values.iter().enumerate() {
                    let SerializableTagValue::Text(value) = value else {
                        unreachable!("text values were validated before writing")
                    };
                    sqlx::query(
                        "INSERT INTO metadata_texts (file_id, key, value, ord)
                         VALUES (?1, ?2, ?3, ?4)",
                    )
                    .bind(file_id)
                    .bind(&key)
                    .bind(value)
                    .bind(ord as i64)
                    .execute(&mut *tx)
                    .await
                    .map_err(|error| error.to_string())?;
                }
            }
        }
    }
    tx.commit().await.map_err(|error| error.to_string())?;

    for file_ids in input.file_ids.chunks(200) {
        app_handle
            .emit("library-changed", LibraryChange { file_ids })
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}
