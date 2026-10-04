use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite};
use tauri::{AppHandle, Emitter};

use crate::file_watcher::FileWatcher;
use crate::tag_manager::tag_backend::{BackendError, DefaultBackend, TagBackend};
use crate::tag_manager::utils::{Changes, FrameKey, SerializableTagValue, TagChange};
use crate::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMetadataInput {
    file_ids: Vec<i64>,
    changes: HashMap<FrameKey, TagChange>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryChange<'a> {
    file_ids: &'a [i64],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileUpdateFailure {
    file_id: i64,
    path: String,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMetadataResult {
    updated_file_ids: Vec<i64>,
    failures: Vec<FileUpdateFailure>,
}

fn error_message(error: &BackendError) -> String {
    let detail = match error {
        BackendError::ReadFailed(error) | BackendError::WriteFailed(error) => error,
    };
    if detail.internal_message == detail.public_message {
        detail.public_message.clone()
    } else {
        format!("{}: {}", detail.public_message, detail.internal_message)
    }
}

#[tauri::command]
pub async fn update_metadata(
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
    input: UpdateMetadataInput,
) -> Result<UpdateMetadataResult, String> {
    if input.file_ids.is_empty() || input.changes.is_empty() {
        return Ok(UpdateMetadataResult {
            updated_file_ids: Vec::new(),
            failures: Vec::new(),
        });
    }

    let mut seen = HashSet::new();
    let file_ids = input
        .file_ids
        .into_iter()
        .filter(|file_id| seen.insert(*file_id))
        .collect::<Vec<_>>();
    if file_ids.len() > 200 {
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
    paths_query.push_bind(file_ids[0]);
    for id in &file_ids[1..] {
        paths_query.push(",").push_bind(id);
    }
    paths_query.push(")");
    let paths_by_id = paths_query
        .build_query_as::<(i64, String)>()
        .fetch_all(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .collect::<HashMap<_, _>>();

    let mut failures = Vec::new();
    let jobs = file_ids
        .iter()
        .filter_map(|file_id| {
            paths_by_id
                .get(file_id)
                .map(|path| (*file_id, path.clone()))
                .or_else(|| {
                    failures.push(FileUpdateFailure {
                        file_id: *file_id,
                        path: String::new(),
                        message: "File no longer exists in the library".into(),
                    });
                    None
                })
        })
        .collect::<Vec<_>>();
    let changes = input.changes;

    let outcomes = tauri::async_runtime::spawn_blocking(move || {
        let backend = DefaultBackend::new();
        jobs.into_iter()
            .map(|(file_id, path)| {
                let errors = backend.write_changes(&Changes {
                    paths: vec![path.clone()],
                    tags: changes.clone(),
                });
                if !errors.is_empty() {
                    let message = errors
                        .iter()
                        .map(error_message)
                        .collect::<Vec<_>>()
                        .join("; ");
                    return (file_id, path, Err(message));
                }

                let metadata = backend.read(Path::new(&path)).map_err(|error| {
                    format!(
                        "Could not reread written metadata: {}",
                        error_message(&error)
                    )
                });
                (file_id, path, metadata)
            })
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|error| error.to_string())?;

    let mut updated_file_ids = Vec::new();
    for (file_id, path, outcome) in outcomes {
        match outcome {
            Ok(metadata) => {
                if let Err(error) =
                    FileWatcher::store_metadata(&state.db.pool, file_id, &metadata).await
                {
                    failures.push(FileUpdateFailure {
                        file_id,
                        path,
                        message: format!(
                            "Metadata was written, but the library cache could not be rebuilt: {error:?}"
                        ),
                    });
                } else {
                    updated_file_ids.push(file_id);
                }
            }
            Err(message) => failures.push(FileUpdateFailure {
                file_id,
                path,
                message,
            }),
        }
    }

    for ids in updated_file_ids.chunks(200) {
        app_handle
            .emit("library-changed", LibraryChange { file_ids: ids })
            .map_err(|error| error.to_string())?;
    }

    Ok(UpdateMetadataResult {
        updated_file_ids,
        failures,
    })
}
