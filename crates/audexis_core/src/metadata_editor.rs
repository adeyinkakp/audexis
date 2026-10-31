use crate::{
    database::Database,
    file_watcher::FileWatcher,
    tag_manager::{
        tag_backend::{DefaultBackend, TagBackend},
        utils::*,
    },
};
use std::{collections::HashMap, path::Path};

#[derive(Clone, Debug)]
pub struct EditFile {
    pub id: i64,
    pub original: MetadataFile,
    pub tags: HashMap<FrameKey, Vec<TagValue>>,
}
impl EditFile {
    pub fn dirty(&self) -> bool {
        !self.changes().is_empty()
    }
    pub fn changes(&self) -> HashMap<FrameKey, TagChange> {
        let mut changes = HashMap::new();
        for key in self.original.tags.keys().chain(self.tags.keys()) {
            let before = self
                .original
                .tags
                .get(key)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let filtered = self
                .tags
                .get(key)
                .into_iter()
                .flatten()
                .filter(|v| {
                    !matches!(v, TagValue::Text(text) if text.is_empty())
                        && !matches!(v, TagValue::Comment {text,..} if text.is_empty())
                })
                .cloned()
                .collect::<Vec<_>>();
            let after = filtered.as_slice();
            if before != after {
                changes.insert(
                    *key,
                    if after.is_empty() {
                        TagChange::Delete
                    } else {
                        TagChange::Replace(TagValuesWrapper(after.to_vec()).into())
                    },
                );
            }
        }
        changes
    }
}
pub fn load(tracks: Vec<(i64, String)>) -> Result<Vec<EditFile>, String> {
    tracks
        .into_iter()
        .map(|(id, path)| {
            let original = DefaultBackend::new()
                .read(Path::new(&path))
                .map_err(|e| format!("{path}: {e:?}"))?;
            Ok(EditFile {
                id,
                tags: original.tags.clone(),
                original,
            })
        })
        .collect()
}
#[derive(Default)]
pub struct SaveResult {
    pub saved: Vec<EditFile>,
    pub failures: Vec<String>,
}
static SAVE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
pub async fn save(db: Database, files: Vec<EditFile>) -> SaveResult {
    let _guard = SAVE_LOCK.lock().await;
    let mut result = SaveResult::default();
    for file in files.into_iter().filter(EditFile::dirty) {
        let id = file.id;
        let path = file.original.path.display().to_string();
        if path.starts_with("cloud-library://") {
            match save_cloud(&db, file).await {
                Ok(file) => result.saved.push(file),
                Err(error) => result.failures.push(format!("{path}: {error}")),
            }
            continue;
        }
        let written = tokio::task::spawn_blocking(move || {
            let changes = file.changes();
            validate_changes(&file)?;
            let backend = DefaultBackend::new();
            let current = backend
                .read(&file.original.path)
                .map_err(|e| format!("{e:?}"))?;
            for key in changes.keys() {
                if current.tags.get(key) != file.original.tags.get(key) {
                    return Err(format!(
                        "{} changed on disk. Reopen Track Info before saving.",
                        key
                    ));
                }
            }
            let errors = backend.write_changes(&Changes {
                paths: vec![file.original.path.display().to_string()],
                tags: changes,
            });
            if !errors.is_empty() {
                return Err(format!("{errors:?}"));
            }
            let original = backend
                .read(&file.original.path)
                .map_err(|e| format!("{e:?}"))?;
            Ok(EditFile {
                id,
                tags: original.tags.clone(),
                original,
            })
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|v| v);
        match written {
            Ok(file) => match FileWatcher::store_metadata(&db.pool, id, &file.original).await {
                Ok(()) => result.saved.push(file),
                Err(error) => {
                    result.failures.push(format!(
                        "{path}: file saved, but library refresh failed: {error}"
                    ));
                    result.saved.push(file);
                }
            },
            Err(error) => result.failures.push(format!("{path}: {error}")),
        }
    }
    result
}

pub use crate::tag_manager::id3::lyrics::{format_lrc, parse_lrc};

pub fn validate_changes(file: &EditFile) -> Result<(), String> {
    use crate::tag_manager::{custom_fields, field_catalog};
    for (key, change) in file.changes() {
        if let TagChange::Replace(values) = change {
            if matches!(key, FrameKey::UserDefinedText | FrameKey::UserDefinedURL) {
                custom_fields::validate_for_format(&file.original.tag_format, key, &values)?;
            }
            if key == FrameKey::SynchronizedLyrics {
                if values.len() != 1 {
                    return Err("Save one synchronized lyrics entry at a time".into());
                }
                if let SerializableTagValue::Text(text) = &values[0] {
                    parse_lrc(text)?;
                }
            }
            if field_catalog::is_text(key) {
                for value in values {
                    let text = match value {
                        SerializableTagValue::Text(s) => s,
                        SerializableTagValue::Comment { text, .. } => text,
                        _ => return Err("Expected text metadata".into()),
                    };
                    if text.contains('\0') {
                        return Err("Metadata cannot contain NUL characters".into());
                    }
                    if matches!(key.get_kind(), FrameKeyKind::URL) && !text.is_ascii() {
                        return Err("URL metadata must contain ASCII characters".into());
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn load_library(
    db: Database,
    tracks: Vec<(i64, String)>,
) -> Result<Vec<EditFile>, String> {
    let mut files = Vec::new();
    for (id, path) in tracks {
        if path.starts_with("cloud-library://") {
            let mut conn = db.pool.acquire().await.map_err(|e| e.to_string())?;
            files.push(load_cloud(&mut conn, id, &path).await?);
        } else {
            files.extend(
                tokio::task::spawn_blocking(move || load(vec![(id, path)]))
                    .await
                    .map_err(|e| e.to_string())??,
            );
        }
    }
    Ok(files)
}
async fn load_cloud(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    path: &str,
) -> Result<EditFile, String> {
    use crate::tag_manager::{field_catalog, traits::Formats};
    let format: String = sqlx::query_scalar("SELECT COALESCE(format,'') FROM files WHERE id=? AND path=? AND cloud_connection_id=(SELECT connection_id FROM cloud_library_session WHERE slot=1) AND missing_since IS NULL")
        .bind(id).bind(path).fetch_optional(&mut *conn).await.map_err(|e| e.to_string())?.ok_or("This song is no longer in the active cloud library")?;
    let format = match format.as_str() {
        "FLAC" => Formats::Flac,
        "Ogg" => Formats::Ogg,
        "Itunes" => Formats::Itunes,
        "id3v2.2" => Formats::Id3v22,
        "id3v2.4" => Formats::Id3v24,
        _ => Formats::Id3v23,
    };
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key,value FROM metadata_texts WHERE file_id=? ORDER BY key,ord")
            .bind(id)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| e.to_string())?;
    let mut tags: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();
    for (key, value) in rows {
        if let Some((frame, _)) = field_catalog::FIELDS
            .iter()
            .find(|(frame, _)| frame.to_string() == key)
        {
            tags.entry(*frame).or_default().push(TagValue::Text(value));
        }
    }
    let pictures: Vec<(String, Vec<u8>, i64, String)> = sqlx::query_as(
        "SELECT mime_type,data,picture_type,description FROM metadata_pictures WHERE file_id=?",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| e.to_string())?;
    for (mime, data, kind, description) in pictures {
        tags.entry(FrameKey::AttachedPicture)
            .or_default()
            .push(TagValue::Picture {
                mime,
                data,
                picture_type: u8::try_from(kind).ok(),
                description: Some(description),
            });
    }
    Ok(EditFile {
        id,
        tags: tags.clone(),
        original: MetadataFile {
            path: path.into(),
            tags,
            tag_format: format.clone(),
            tag_formats: vec![format],
            freeforms: vec![],
        },
    })
}
async fn save_cloud(db: &Database, file: EditFile) -> Result<EditFile, String> {
    validate_changes(&file)?;
    let changes = file.changes();
    if changes.keys().any(|key| {
        matches!(
            key,
            FrameKey::AttachedPicture | FrameKey::UserDefinedText | FrameKey::UserDefinedURL
        )
    }) {
        return Err("Cloud artwork and custom fields cannot be changed in this editor yet.".into());
    }
    let path = file.original.path.display().to_string();
    let mut tx = db
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| e.to_string())?;
    let current = load_cloud(&mut tx, file.id, &path).await?;
    for key in changes.keys() {
        if current.tags.get(key) != file.original.tags.get(key) {
            return Err(format!(
                "{key} changed in the cloud. Reopen Track Info before saving."
            ));
        }
        sqlx::query("DELETE FROM metadata_texts WHERE file_id=? AND key=?")
            .bind(file.id)
            .bind(key.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        if let Some(values) = file.tags.get(key) {
            for (ord, value) in values.iter().enumerate() {
                let text = match value {
                    TagValue::Text(text) | TagValue::Comment { text, .. } => text.clone(),
                    _ => value.to_string(),
                };
                sqlx::query("INSERT INTO metadata_texts(file_id,key,value,ord) VALUES (?,?,?,?)")
                    .bind(file.id)
                    .bind(key.to_string())
                    .bind(text)
                    .bind(ord as i64)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    use crate::cloud_sync::{
        protocol::{Edit, Mutation},
        store::{self, Job},
    };
    let (connection, track_id, revision): (String,String,i64) = sqlx::query_as("SELECT l.connection_id,l.cloud_track_id,COALESCE(s.revision,c.revision) FROM file_cloud_links l JOIN cloud_library_session active ON active.connection_id=l.connection_id JOIN cloud_tracks c ON c.connection_id=l.connection_id AND c.id=l.cloud_track_id LEFT JOIN cloud_local_state s ON s.connection_id=l.connection_id AND s.file_id=l.file_id WHERE l.file_id=?")
        .bind(file.id).fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
    let metadata = store::local_metadata(&mut tx, file.id)
        .await
        .map_err(|e| e.to_string())?;
    metadata.validate()?;
    let operation_id = uuid::Uuid::new_v4().to_string();
    let job = Job::Mutate {
        file_id: Some(file.id),
        playlist_id: None,
        request: Mutation {
            operation_id: operation_id.clone(),
            base_revision: revision,
            edit: Edit::Track { track_id, metadata },
        },
    };
    let inserted = sqlx::query("INSERT INTO cloud_outbox(operation_id,connection_id,entity_key,payload) VALUES (?,?,?,?) ON CONFLICT(connection_id,entity_key) DO NOTHING")
        .bind(operation_id).bind(connection).bind(format!("track:{}",file.id)).bind(serde_json::to_string(&job).map_err(|e|e.to_string())?).execute(&mut *tx).await.map_err(|e|e.to_string())?;
    if inserted.rows_affected() == 0 {
        return Err("An edit to this song is already syncing. Wait for it to finish or resolve its sync error before saving again.".into());
    }
    let saved = load_cloud(&mut tx, file.id, &path).await?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(saved)
}
