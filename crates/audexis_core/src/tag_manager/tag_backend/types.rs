use super::*;

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
        crate::library_files::check_after_error(Path::new(&error.path));
        tracing::error!(path = %error.path, message = %error.public_message,
            detail = %error.internal_message, "Tag backend error");
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
    pub(super) fn from_change(
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
