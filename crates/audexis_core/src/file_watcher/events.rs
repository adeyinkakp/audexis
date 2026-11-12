use super::{WatcherContext, WatcherError};
#[derive(Clone, Debug, Default)]
pub struct WatcherStatus {
    pub busy: bool,
    pub message: String,
    pub processed: u64,
    pub total: Option<u64>,
    pub error: Option<String>,
}
pub(super) fn emit_changed(context: &WatcherContext, ids: &[i64]) -> Result<(), WatcherError> {
    if !ids.is_empty() {
        context.files.publish_changed(ids.to_vec());
    }
    Ok(())
}
pub(super) struct TaskGuard {
    status: tokio::sync::watch::Sender<WatcherStatus>,
}
impl TaskGuard {
    pub fn start(context: &WatcherContext, _id: &str, title: &str, total: Option<u64>) -> Self {
        context.status.send_modify(|s| {
            s.busy = true;
            s.message = title.into();
            s.processed = 0;
            s.total = total;
        });
        Self {
            status: context.status.clone(),
        }
    }
    pub fn update(&self, processed: u64, total: Option<u64>, message: Option<String>) {
        self.status.send_modify(|s| {
            s.processed = processed;
            s.total = total;
            if let Some(message) = message {
                s.message = message;
            }
        });
    }
    pub fn complete(self, message: String) {
        self.status.send_modify(|s| s.message = message);
    }
}
impl Drop for TaskGuard {
    fn drop(&mut self) {
        self.status.send_modify(|s| s.busy = false);
    }
}
