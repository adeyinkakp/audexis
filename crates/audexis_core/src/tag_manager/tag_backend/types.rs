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
