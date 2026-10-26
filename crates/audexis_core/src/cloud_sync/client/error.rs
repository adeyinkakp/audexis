#[derive(Debug)]
pub struct SyncError {
    pub status: Option<u16>,
    pub message: String,
    pub retry_after: u64,
}
impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}
impl std::error::Error for SyncError {}
impl From<sqlx::Error> for SyncError {
    fn from(e: sqlx::Error) -> Self {
        Self::local(format!(
            "Cloud sync database: {e}. Apply the cloud-sync schema if needed."
        ))
    }
}
impl From<serde_json::Error> for SyncError {
    fn from(_: serde_json::Error) -> Self {
        Self::local("Invalid saved cloud sync data")
    }
}
impl From<std::io::Error> for SyncError {
    fn from(e: std::io::Error) -> Self {
        Self::local(format!("Cloud file I/O: {e}"))
    }
}
impl SyncError {
    pub fn local(message: impl Into<String>) -> Self {
        Self {
            status: None,
            message: message.into(),
            retry_after: 0,
        }
    }
}
pub type Result<T> = std::result::Result<T, SyncError>;
