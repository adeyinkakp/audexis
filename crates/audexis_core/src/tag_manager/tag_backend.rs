use super::TagManager;
use super::traits::{Formats, TagFormat};
use super::utils::{Changes, FrameKey, MetadataFile, SerializableTagValue, TagChange, TagValue};
use base64::Engine;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
pub trait TagBackend {
    fn read(&self, path: &Path) -> Result<MetadataFile, BackendError>;

    fn write_changes(&self, changes: &Changes) -> Vec<BackendError>;
}

#[derive(Debug, Clone)]
pub struct DefaultBackend {
    manager: TagManager,
}

impl Default for DefaultBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultBackend {
    pub fn new() -> Self {
        Self {
            manager: TagManager::new(),
        }
    }

    pub fn resolve_format(&self, path: &Path) -> Formats {
        self.manager.detect_tag_format(path)
    }

    fn resolve_release(&self, fmt: &Formats) -> Option<Box<dyn TagFormat>> {
        self.manager.get_release_class(fmt)
    }

    fn has_no_id3_header(path: &Path, format: &Formats) -> bool {
        use std::io::Read;
        if *format != Formats::Id3v23 {
            return false;
        }
        let mut signature = [0; 3];
        crate::library_files::open(path)
            .and_then(|mut file| file.read_exact(&mut signature))
            .is_ok()
            && &signature != b"ID3"
    }

    pub fn detect_all_formats(&self, path: &Path, primary: &Formats) -> Vec<Formats> {
        super::detect_formats(path, Some(primary.clone()))
    }

    fn write_verified(
        &self,
        format: &Formats,
        release: &dyn TagFormat,
        path: &PathBuf,
        updated: HashMap<FrameKey, Vec<TagValue>>,
    ) -> Result<(), String> {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "File has no usable name".to_string())?;
        if matches!(format, Formats::Id3v22 | Formats::Id3v23 | Formats::Id3v24)
            && !Self::has_no_id3_header(path, format)
        {
            super::id3::v2_common::validate_for_write(path)?;
        }
        let temporary_name = format!(".{file_name}.audexis-tmp-{}", uuid::Uuid::new_v4());
        let temporary_path = path.with_file_name(temporary_name);

        fs::copy(path, &temporary_path).map_err(|error| error.to_string())?;
        let result = (|| {
            if Self::has_no_id3_header(path, format) {
                super::id3::ensure_v23_header(&temporary_path)
                    .map_err(|error| error.to_string())?;
            }
            let existing = release
                .get_tags(&temporary_path)
                .map_err(|error| format!("Could not read existing fields: {error:?}"))?;
            let updated = super::portable_fields::prepare_changes(format, &updated, &existing)?;
            release
                .write_tags(&temporary_path, updated.clone())
                .map_err(|error| format!("Codec write failed: {error:?}"))?;
            let written = release
                .get_tags(&temporary_path)
                .map_err(|error| format!("Written metadata could not be verified: {error:?}"))?;
            for (key, values) in &updated {
                if values.is_empty() {
                    if written.get(key).is_some_and(|written| !written.is_empty()) {
                        return Err(format!("Codec did not delete {}", key));
                    }
                } else if !values_preserved(format, *key, values, written.get(key)) {
                    return Err(format!("Codec did not preserve {}", key));
                }
            }
            fs::rename(&temporary_path, path).map_err(|error| error.to_string())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        result
    }
}

impl TagBackend for DefaultBackend {
    /// Reads the tags from the specified file path and returns a `File` struct containing the tag information.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to a `PathBuf` representing the file path to read tags from.
    ///
    /// # Returns
    ///
    /// * `Ok(File)` - If the tags were successfully read, returns a `File` struct containing the tag information.
    /// * `Err(BackendError)` - If there was an error reading the tags, returns a `BackendError` with details about the failure.
    fn read(&self, path: &Path) -> Result<MetadataFile, BackendError> {
        let tag_formats = super::detect_formats(path, None);
        let fmt = tag_formats.first().cloned().unwrap_or(Formats::Unknown);

        let release = self.resolve_release(&fmt).ok_or_else(|| {
            BackendError::ReadFailed(TagError {
                path: path.to_string_lossy().to_string(),
                public_message: "Unsupported format".to_string(),
                internal_message: "Could not resolve tag format for reading".to_string(),
            })
        })?;
        let (mut tag_map, freeforms) = if Self::has_no_id3_header(path, &fmt) {
            (HashMap::new(), Vec::new())
        } else {
            release.get_metadata(path)?
        };
        super::portable_fields::expose_regular_fields(&fmt, &mut tag_map);
        Ok(MetadataFile {
            path: path.to_path_buf(),
            tags: tag_map,
            tag_format: fmt,
            tag_formats,
            freeforms,
        })
    }

    /// Writes the specified tag changes to the corresponding files and returns a vector of `BackendError` instances for any failures that occur during the write process.
    ///
    /// # Arguments
    ///
    /// * `changes` - A reference to a `Changes` struct containing the tag changes to be written.
    ///
    /// # Returns
    ///
    /// * `Vec<BackendError>` - A vector of `BackendError` instances representing any errors that occurred during the write process. If the vector is empty, it indicates that all changes were successfully written.
    fn write_changes(&self, changes: &Changes) -> Vec<BackendError> {
        let mut results: Vec<BackendError> = Vec::new();
        for path_str in &changes.paths {
            let path = PathBuf::from(path_str);
            let fmt = self.resolve_format(&path);
            if changes
                .tags
                .keys()
                .any(|key| matches!(key, FrameKey::UserDefinedText | FrameKey::UserDefinedURL))
            {
                let validation = if !super::custom_fields::supported(&fmt) {
                    Err("This metadata format does not support custom fields".to_string())
                } else {
                    changes.tags.iter().try_for_each(|(key, change)| {
                        if let TagChange::Replace(values) = change {
                            super::custom_fields::validate_for_format(&fmt, *key, values)?;
                        }
                        Ok(())
                    })
                };
                if let Err(message) = validation {
                    results.push(BackendError::WriteFailed(TagError {
                        path: path_str.clone(),
                        public_message: "Invalid custom fields".into(),
                        internal_message: message,
                    }));
                    continue;
                }
            }
            let Some(release) = self.resolve_release(&fmt) else {
                results.push(BackendError::WriteFailed(TagError {
                    path: path_str.clone(),
                    public_message: "Unsupported format".to_string(),
                    internal_message: "Could not resolve tag format for writing".to_string(),
                }));
                continue;
            };
            let updated = changes
                .tags
                .iter()
                .map(|(key, change)| {
                    let values: &[SerializableTagValue] = match change {
                        TagChange::Replace(values) => values,
                        TagChange::Delete => &[],
                    };
                    values
                        .iter()
                        .map(to_tag_value)
                        .collect::<Result<Vec<_>, _>>()
                        .map(|values| (*key, values))
                })
                .collect::<Result<HashMap<FrameKey, Vec<TagValue>>, _>>();
            let updated = match updated {
                Ok(updated) => updated,
                Err(message) => {
                    results.push(BackendError::WriteFailed(TagError {
                        path: path_str.clone(),
                        public_message: "Invalid tag value".to_string(),
                        internal_message: message,
                    }));
                    continue;
                }
            };
            let existing_tags = match self.read(&path).map(|metadata| metadata.tags) {
                Ok(m) => m,
                Err(error) => {
                    results.push(BackendError::WriteFailed(TagError {
                        path: path_str.clone(),
                        public_message: "Failed to read existing tags".to_string(),
                        internal_message: format!("{error:?}"),
                    }));
                    continue;
                }
            };
            let mut diffs: Vec<TagDiff> = Vec::new();
            for (k, new_vals) in &updated {
                let old_vals = existing_tags.get(k).cloned();
                if old_vals.as_ref() != Some(new_vals) {
                    diffs.push(TagDiff::from_change(
                        *k,
                        old_vals.clone(),
                        Some(new_vals.clone()),
                    ));
                }
            }

            let write_res = self.write_verified(&fmt, release.as_ref(), &path, updated);
            if let Err(message) = write_res {
                results.push(BackendError::WriteFailed(TagError {
                    path: path_str.clone(),
                    public_message: "Failed to write tags".to_string(),
                    internal_message: message,
                }));
                continue;
            }
        }
        results
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "tag_backend_tests.rs"]
mod round_trip_tests;

mod types;
mod validation;
pub use types::*;
use validation::{to_tag_value, values_preserved};
