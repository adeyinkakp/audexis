use super::traits::{Formats, TagFormat};
use super::utils::{Changes, FrameKey, MetadataFile, SerializableTagValue, TagChange, TagValue};
use super::TagManager;
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
        crate::utils::library_files::open(path)
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

fn values_preserved(
    format: &Formats,
    key: FrameKey,
    expected: &[TagValue],
    actual: Option<&Vec<TagValue>>,
) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    if *format == Formats::Itunes && matches!(key, FrameKey::TrackNumber | FrameKey::DiscNumber) {
        let ([TagValue::Text(expected)], [TagValue::Text(actual)]) = (expected, actual.as_slice())
        else {
            return false;
        };
        return match (
            super::itunes::utils::parse_number_pair(expected),
            super::itunes::utils::parse_number_pair(actual),
        ) {
            (Some((number, total)), Some((written_number, written_total))) => {
                number == written_number
                    && total.is_none_or(|total| total == written_total.unwrap_or(0))
            }
            _ => false,
        };
    }
    if key == FrameKey::SynchronizedLyrics {
        let parse = |values: &[TagValue]| {
            values
                .iter()
                .map(|value| match value {
                    TagValue::Text(text) => super::id3::lyrics::parse_lrc(text),
                    _ => Err("Expected lyrics text".into()),
                })
                .collect::<Result<Vec<_>, String>>()
        };
        return matches!((parse(expected), parse(actual)), (Ok(a), Ok(b)) if a == b);
    }
    if key != FrameKey::AttachedPicture {
        return actual == expected;
    }
    if actual.len() != expected.len() {
        return false;
    }

    expected.iter().zip(actual).all(|(expected, actual)| {
        let (
            TagValue::Picture {
                mime: expected_mime,
                data: expected_data,
                picture_type: expected_type,
                description: expected_description,
            },
            TagValue::Picture {
                mime: actual_mime,
                data: actual_data,
                picture_type: actual_type,
                description: actual_description,
            },
        ) = (expected, actual)
        else {
            return false;
        };

        let is_jpeg = |mime: &str| {
            mime.eq_ignore_ascii_case("image/jpg") || mime.eq_ignore_ascii_case("image/jpeg")
        };
        let mime_matches = expected_mime.eq_ignore_ascii_case(actual_mime)
            || (is_jpeg(expected_mime) && is_jpeg(actual_mime));
        let image_matches = mime_matches && expected_data == actual_data;
        if *format == Formats::Itunes {
            return image_matches;
        }

        image_matches
            && expected_type.unwrap_or(3) == actual_type.unwrap_or(3)
            && expected_description.as_deref().unwrap_or_default()
                == actual_description.as_deref().unwrap_or_default()
    })
}

fn to_tag_value(value: &SerializableTagValue) -> Result<TagValue, String> {
    match value {
        SerializableTagValue::Text(text) => Ok(TagValue::Text(text.clone())),
        SerializableTagValue::Picture {
            mime,
            data_base64,
            picture_type,
            description,
        } => base64::engine::general_purpose::STANDARD
            .decode(data_base64)
            .map(|data| TagValue::Picture {
                mime: mime.clone(),
                data,
                picture_type: *picture_type,
                description: description.clone(),
            })
            .map_err(|error| format!("Invalid picture data: {error}")),
        SerializableTagValue::UserText(entry) => Ok(TagValue::UserText(entry.clone())),
        SerializableTagValue::UserUrl(entry) => Ok(TagValue::UserUrl(entry.clone())),
        SerializableTagValue::Comment {
            encoding,
            language,
            description,
            text,
        } => Ok(TagValue::Comment {
            encoding: encoding.clone(),
            language: language.clone(),
            description: description.clone(),
            text: text.clone(),
        }),
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
        let fmt = self.resolve_format(path);

        let release = self.resolve_release(&fmt).ok_or_else(|| {
            BackendError::ReadFailed(TagError {
                path: path.to_string_lossy().to_string(),
                public_message: "Unsupported format".to_string(),
                internal_message: "Could not resolve tag format for reading".to_string(),
            })
        })?;
        let (tag_map, freeforms) = if Self::has_no_id3_header(path, &fmt) {
            (HashMap::new(), Vec::new())
        } else {
            (release.get_tags(path)?, release.get_freeforms(path)?)
        };
        let tag_formats = self.detect_all_formats(path, &fmt);
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

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum BackendError {
    ReadFailed(TagError),
    WriteFailed(TagError),
}

impl Drop for BackendError {
    fn drop(&mut self) {
        let error = match self {
            Self::ReadFailed(error) | Self::WriteFailed(error) => error,
        };
        crate::utils::library_files::check_after_error(Path::new(&error.path));
        crate::utils::errors::report_registered_backend_error(
            "BackendError",
            &error.public_message,
            format!(
                "{} | {} | {}",
                error.path, error.public_message, error.internal_message
            ),
        );
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct TagError {
    pub path: String,
    pub public_message: String,
    pub internal_message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagDiff {
    pub key: String,
    pub before: Option<Vec<SerializableTagValue>>,
    pub after: Option<Vec<SerializableTagValue>>,
}

impl TagDiff {
    /// Creates a `TagDiff` instance from the given `FrameKey`, optional `before` values, and optional `after` values.
    fn from_change(
        key: FrameKey,
        before: Option<Vec<TagValue>>,
        after: Option<Vec<TagValue>>,
    ) -> Self {
        let key_str = key.to_string();
        let conv_vec = |opt_vec: Option<Vec<TagValue>>| -> Option<Vec<SerializableTagValue>> {
            opt_vec.map(|vec| {
                vec.into_iter()
                    .map(|v| match v {
                        TagValue::Text(s) => SerializableTagValue::Text(s),
                        TagValue::Picture {
                            mime,
                            data,
                            picture_type,
                            description,
                        } => SerializableTagValue::Picture {
                            mime,
                            data_base64: base64::engine::general_purpose::STANDARD.encode(&data),
                            picture_type,
                            description,
                        },
                        TagValue::UserText(item) => SerializableTagValue::UserText(item),
                        TagValue::UserUrl(item) => SerializableTagValue::UserUrl(item),
                        TagValue::Comment {
                            encoding,
                            language,
                            description,
                            text,
                        } => SerializableTagValue::Comment {
                            encoding,
                            language,
                            description,
                            text,
                        },
                    })
                    .collect()
            })
        };
        TagDiff {
            key: key_str,
            before: conv_vec(before),
            after: conv_vec(after),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(mime: &str, picture_type: Option<u8>, description: Option<&str>) -> TagValue {
        TagValue::Picture {
            mime: mime.to_string(),
            data: vec![1, 2, 3, 4],
            picture_type,
            description: description.map(str::to_string),
        }
    }

    #[test]
    fn artwork_verification_normalizes_empty_optional_fields() {
        let expected = vec![picture("image/jpeg", Some(3), Some(""))];
        let actual = vec![picture("image/jpg", None, None)];

        assert!(values_preserved(
            &Formats::Id3v23,
            FrameKey::AttachedPicture,
            &expected,
            Some(&actual),
        ));
    }

    #[test]
    fn itunes_artwork_verification_ignores_unsupported_fields() {
        let expected = vec![picture("image/png", Some(4), Some("Back cover"))];
        let actual = vec![picture("image/png", None, None)];

        assert!(values_preserved(
            &Formats::Itunes,
            FrameKey::AttachedPicture,
            &expected,
            Some(&actual),
        ));
    }

    #[test]
    fn artwork_verification_still_rejects_changed_image_data() {
        let expected = vec![picture("image/png", Some(3), None)];
        let mut actual = vec![picture("image/png", Some(3), None)];
        if let TagValue::Picture { data, .. } = &mut actual[0] {
            data.push(5);
        }

        assert!(!values_preserved(
            &Formats::Flac,
            FrameKey::AttachedPicture,
            &expected,
            Some(&actual),
        ));
    }
}

#[cfg(test)]
#[path = "tag_backend_tests.rs"]
mod round_trip_tests;
