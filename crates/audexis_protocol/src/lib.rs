use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Tags = BTreeMap<String, Vec<String>>;
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    #[serde(default)]
    pub tags: Tags,
    #[serde(default)]
    pub loved: bool,
}
impl Metadata {
    pub fn for_sync(&self) -> Self {
        let mut tags = Tags::new();
        for (key, values) in &self.tags {
            tags.entry(key.replace('\0', ""))
                .or_default()
                .extend(values.iter().map(|value| value.replace('\0', "")));
        }
        Self {
            tags,
            loved: self.loved,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.tags.len() > 1024 {
            return Err(format!(
                "Metadata has {} fields; maximum is 1024",
                self.tags.len()
            ));
        }
        let mut total = 0usize;
        for (key, values) in &self.tags {
            if key.len() > 1024 || key.contains('\0') {
                return Err(
                    "Metadata field name exceeds 1024 bytes or contains a null character".into(),
                );
            }
            if values.len() > 256 {
                return Err(format!(
                    "Metadata field {key:?} has {} values; maximum is 256",
                    values.len()
                ));
            }
            total += key.len();
            for value in values {
                if value.contains('\0') {
                    return Err(format!("Metadata field {key:?} contains a null character"));
                }
                if value.len() > 256 * 1024 {
                    return Err(format!(
                        "Metadata field {key:?} is {} bytes; maximum per value is 262144 bytes",
                        value.len()
                    ));
                }
                total += value.len();
            }
        }
        if total > 512 * 1024 {
            return Err(format!(
                "Metadata totals {total} bytes; maximum per track is 524288 bytes"
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub id: String,
    pub revision: i64,
    pub sha256: String,
    pub size: i64,
    pub file_name: String,
    pub format: String,
    pub duration_ms: Option<i64>,
    pub metadata: Metadata,
    pub artwork: bool,
    #[serde(default)]
    pub deleted: bool,
}
impl Track {
    pub fn title(&self) -> String {
        self.metadata
            .tags
            .get("title")
            .and_then(|v| v.first())
            .filter(|s| !s.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| self.file_name.clone())
    }
    pub fn artist(&self) -> String {
        self.metadata
            .tags
            .get("artist")
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Playlist {
    pub id: String,
    pub revision: i64,
    pub name: String,
    pub track_ids: Vec<String>,
    #[serde(default)]
    pub deleted: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ChangeValue {
    Track(Track),
    Playlist(Playlist),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Change {
    pub cursor: i64,
    pub value: ChangeValue,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Changes {
    pub changes: Vec<Change>,
    pub next_cursor: i64,
    pub has_more: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadRequest {
    pub operation_id: String,
    pub sha256: String,
    pub size: i64,
    pub file_name: String,
    pub metadata: Metadata,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Upload {
    pub id: String,
    pub state: String,
    pub upload_url: Option<String>,
    pub track: Option<Track>,
    pub error: Option<String>,
    #[serde(default)]
    pub error_status: Option<u16>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Edit {
    Track {
        track_id: String,
        metadata: Metadata,
    },
    DeleteTrack {
        track_id: String,
    },
    Playlist {
        playlist_id: String,
        name: String,
        track_ids: Vec<String>,
    },
    DeletePlaylist {
        playlist_id: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mutation {
    pub operation_id: String,
    pub base_revision: i64,
    pub edit: Edit,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MutationResult {
    pub value: ChangeValue,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaUrl {
    pub url: String,
    pub sha256: String,
    pub size: i64,
    #[serde(default = "original_quality")]
    pub quality: String,
    #[serde(default)]
    pub preparing: bool,
}
fn original_quality() -> String {
    "original".into()
}

pub fn is_storage_cap(code: &str, message: &str) -> bool {
    matches!(
        code.to_ascii_lowercase().as_str(),
        "download_cap_exceeded" | "transaction_cap_exceeded" | "cap_exceeded"
    ) || [
        "download cap exceeded",
        "transaction cap exceeded",
        "download_cap_exceeded",
        "transaction_cap_exceeded",
        "cap exceeded",
        "cap_exceeded",
    ]
    .iter()
    .any(|needle| message.to_ascii_lowercase().contains(needle))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Usage {
    pub used_bytes: i64,
    pub reserved_bytes: i64,
    pub quota_bytes: i64,
}
