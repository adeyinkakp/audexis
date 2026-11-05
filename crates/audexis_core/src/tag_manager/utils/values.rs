use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagValue {
    Text(String),
    Picture {
        mime: String,
        data: Vec<u8>,
        picture_type: Option<u8>,
        description: Option<String>,
    },
    UserText(UserTextEntry),
    UserUrl(UserUrlEntry),
    Comment {
        encoding: String,
        language: String,
        description: String,
        text: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PictureData {
    pub mime: String,
    pub data: Vec<u8>,
    pub picture_type: Option<u8>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserTextEntry {
    pub description: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserUrlEntry {
    pub description: String,
    pub url: String,
}

pub type TagMap = HashMap<String, Vec<TagValue>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FreeformTag {
    pub mean: String,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
/// File struct that can be sent to frontend via ipc
pub struct SerializableFile {
    pub path: String,
    pub file_name: String,
    pub tag_format: String,
    pub tag_formats: Vec<String>,
    pub tags: HashMap<String, Vec<SerializableTagValue>>,
    pub freeforms: Vec<SerializableFreeform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", content = "values", rename_all = "camelCase")]
pub enum TagChange {
    Replace(Vec<SerializableTagValue>),
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Changes {
    pub paths: Vec<String>,
    pub tags: HashMap<FrameKey, TagChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableTagFrame {
    pub key: FrameKey,
    pub values: Vec<SerializableTagValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameChanges {
    pub paths: Vec<String>,
    pub frames: Vec<SerializableTagFrame>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SerializableTagValue {
    Text(String),
    Picture {
        mime: String,
        data_base64: String,
        picture_type: Option<u8>,
        description: Option<String>,
    },
    UserText(UserTextEntry),
    UserUrl(UserUrlEntry),
    Comment {
        encoding: String,
        language: String,
        description: String,
        text: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupRule {
    ReplaceUnderscores,
    NormalizeDashes,
    FixCapitalization,
    TrimWhitespace,
    CollapseSpaces,

    RemoveSuffixes,
    NormalizeFeat,
    RemoveBrackets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableFreeform {
    pub mean: String,
    pub name: String,
    pub value: String,
}

/// Internal File Struct
#[derive(Debug, Clone)]
pub struct MetadataFile {
    pub path: PathBuf,

    pub tags: HashMap<FrameKey, Vec<TagValue>>,
    pub tag_format: Formats,
    pub tag_formats: Vec<Formats>,
    pub freeforms: Vec<FreeformTag>,
}

impl fmt::Display for TagValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TagValue::Text(s) => write!(f, "{}", s),
            TagValue::Picture {
                mime,
                data,
                picture_type,
                description,
            } => {
                let t = picture_type
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".to_string());
                let desc = description.clone().unwrap_or_default();
                write!(
                    f,
                    "Picture (type {}, desc '{}', MIME: {}, {} bytes)",
                    t,
                    desc,
                    mime,
                    data.len()
                )
            }
            TagValue::UserText(ut) => write!(f, "{}={}", ut.description, ut.value),
            TagValue::UserUrl(uu) => write!(f, "{}={}", uu.description, uu.url),
            TagValue::Comment {
                encoding,
                language,
                description,
                text,
            } => write!(
                f,
                "Comment (encoding: {}, language: {}, description: {}, text: {})",
                encoding, language, description, text
            ),
        }
    }
}
