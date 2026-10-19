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
