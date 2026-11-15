use crate::{
    database::Database,
    file_watcher::FileWatcher,
    tag_manager::{
        tag_backend::{DefaultBackend, TagBackend},
        utils::*,
    },
};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Debug)]
pub enum EditTags {
    Original(Arc<MetadataFile>),
    Edited(Arc<HashMap<FrameKey, Vec<TagValue>>>),
}

impl From<HashMap<FrameKey, Vec<TagValue>>> for EditTags {
    fn from(tags: HashMap<FrameKey, Vec<TagValue>>) -> Self {
        Self::Edited(Arc::new(tags))
    }
}
impl Deref for EditTags {
    type Target = HashMap<FrameKey, Vec<TagValue>>;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Original(file) => &file.tags,
            Self::Edited(tags) => tags,
        }
    }
}
impl DerefMut for EditTags {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let Self::Original(file) = self {
            *self = Self::Edited(Arc::new(file.tags.clone()));
        }
        match self {
            Self::Edited(tags) => Arc::make_mut(tags),
            Self::Original(_) => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EditFile {
    pub id: i64,
    pub original: Arc<MetadataFile>,
    pub tags: EditTags,
}
impl EditFile {
    pub fn from_metadata(id: i64, original: MetadataFile) -> Self {
        let original = Arc::new(original);
        Self {
            id,
            tags: EditTags::Original(original.clone()),
            original,
        }
    }
    pub fn dirty(&self) -> bool {
        self.changed_keys().next().is_some()
    }
    pub fn changed_keys(&self) -> impl Iterator<Item = FrameKey> + '_ {
        self.changed_keys_matching(|_| true)
    }
    pub fn changed_keys_matching(
        &self,
        includes: impl Fn(FrameKey) -> bool,
    ) -> impl Iterator<Item = FrameKey> {
        let untouched =
            matches!(&self.tags, EditTags::Original(file) if Arc::ptr_eq(file, &self.original));
        self.original
            .tags
            .keys()
            .chain(
                self.tags
                    .keys()
                    .filter(|key| !self.original.tags.contains_key(key)),
            )
            .copied()
            .filter(move |key| {
                if untouched || !includes(*key) {
                    return false;
                }
                let before = self.original.tags.get(key).into_iter().flatten();
                let after = self.tags.get(key).into_iter().flatten().filter(|value| {
                    !matches!(value, TagValue::Text(text) if text.is_empty())
                        && !matches!(value, TagValue::Comment {text, ..} if text.is_empty())
                });
                !before.eq(after)
            })
    }
    pub fn changes(&self) -> HashMap<FrameKey, TagChange> {
        self.changes_matching(|_| true)
    }
    pub fn changes_matching(
        &self,
        includes: impl Fn(FrameKey) -> bool,
    ) -> HashMap<FrameKey, TagChange> {
        self.changed_keys_matching(includes)
            .map(|key| {
                let values: Vec<_> = self
                    .tags
                    .get(&key)
                    .into_iter()
                    .flatten()
                    .filter(|value| {
                        !matches!(value, TagValue::Text(text) if text.is_empty())
                            && !matches!(value, TagValue::Comment {text, ..} if text.is_empty())
                    })
                    .cloned()
                    .collect();
                (
                    key,
                    if values.is_empty() {
                        TagChange::Delete
                    } else {
                        TagChange::Replace(TagValuesWrapper(values).into())
                    },
                )
            })
            .collect()
    }
}
pub fn load(tracks: Vec<(i64, String)>) -> Result<Vec<EditFile>, String> {
    tracks
        .into_iter()
        .map(|(id, path)| {
            let original = DefaultBackend::new()
                .read(Path::new(&path))
                .map_err(|e| format!("{path}: {e:?}"))?;
            Ok(EditFile::from_metadata(id, original))
        })
        .collect()
}
#[derive(Default)]
pub struct SaveResult {
    pub attempted: usize,
    pub saved: Vec<EditFile>,
    pub failures: Vec<String>,
}
pub(crate) static SAVE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
pub async fn save(db: Database, files: Vec<EditFile>) -> SaveResult {
    save_cancellable(db, files, Arc::new(AtomicBool::new(false))).await
}
pub async fn save_cancellable(
    db: Database,
    files: Vec<EditFile>,
    cancelled: Arc<AtomicBool>,
) -> SaveResult {
    save_with_progress(db, files, cancelled, |_| {}).await
}

pub async fn save_with_progress(
    db: Database,
    files: Vec<EditFile>,
    cancelled: Arc<AtomicBool>,
    mut progress: impl FnMut(&SaveResult),
) -> SaveResult {
    let _guard = SAVE_LOCK.lock().await;
    let mut pending: Vec<_> = files.into_iter().filter(EditFile::dirty).collect();
    let mut active = std::collections::HashSet::new();
    let mut workers = tokio::task::JoinSet::new();
    let mut result = SaveResult::default();
    loop {
        while workers.len() < 2 && !cancelled.load(Ordering::Acquire) {
            let Some(index) = pending
                .iter()
                .position(|file| !active.contains(&file.original.path))
            else {
                break;
            };
            let file = pending.remove(index);
            let path = file.original.path.clone();
            active.insert(path.clone());
            let db = db.clone();
            workers.spawn(async move { (path, save_one(db, file).await) });
        }
        let Some(next) = workers.join_next().await else {
            break;
        };
        let next = match next {
            Ok((path, next)) => {
                active.remove(&path);
                next
            }
            Err(error) => SaveResult {
                attempted: 1,
                failures: vec![format!("Save worker stopped: {error}")],
                ..Default::default()
            },
        };
        progress(&next);
        result.attempted += next.attempted;
        result.saved.extend(next.saved);
        result.failures.extend(next.failures);
    }
    result
}

async fn save_one(db: Database, file: EditFile) -> SaveResult {
    let started = std::time::Instant::now();
    let mut result = SaveResult {
        attempted: 1,
        ..Default::default()
    };
    let id = file.id;
    let path = file.original.path.display().to_string();
    if path.starts_with("cloud-library://") {
        match save_cloud(&db, file).await {
            Ok(file) => result.saved.push(file),
            Err(error) => result.failures.push(format!("{path}: {error}")),
        }
        return result;
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
        let stamp = crate::file_watcher::saved_edits::Stamp::read(&original.path);
        Ok((EditFile::from_metadata(id, original), stamp))
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|v| v);
    match written {
        Ok((file, stamp)) => {
            match FileWatcher::store_edited_metadata(&db.pool, id, &file.original).await {
                Ok(()) => {
                    crate::file_watcher::saved_edits::remember(&file.original.path, stamp);
                    result.saved.push(file);
                }
                Err(error) => {
                    result.failures.push(format!(
                        "{path}: file saved, but library refresh failed: {error}"
                    ));
                    result.saved.push(file);
                }
            }
        }
        Err(error) => result.failures.push(format!("{path}: {error}")),
    }
    tracing::debug!(
        file_id = id,
        elapsed_ms = started.elapsed().as_millis(),
        "Saved metadata"
    );
    result
}

pub use crate::tag_manager::id3::lyrics::{format_lrc, parse_lrc};

pub fn validate_changes(file: &EditFile) -> Result<(), String> {
    validate_changes_matching(file, |_| true)
}

pub fn validate_changes_matching(
    file: &EditFile,
    includes: impl Fn(FrameKey) -> bool,
) -> Result<(), String> {
    use crate::tag_manager::{custom_fields, field_catalog};
    for (key, change) in file.changes_matching(includes) {
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

mod cloud;
pub use cloud::load_library;
use cloud::save_cloud;
