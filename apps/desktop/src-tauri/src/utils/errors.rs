use tauri::{AppHandle, Emitter};
use thiserror::Error;

static APP_HANDLE: once_cell::sync::OnceCell<AppHandle> = once_cell::sync::OnceCell::new();

pub fn init_error_reporting(app: &AppHandle) {
    let _ = APP_HANDLE.set(app.clone());
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendErrorEvent {
    pub kind: String,
    pub message: String,
    pub details: String,
}

pub fn report_backend_error(
    app: &AppHandle,
    kind: impl Into<String>,
    message: impl Into<String>,
    details: impl Into<String>,
) {
    let event = BackendErrorEvent {
        kind: kind.into(),
        message: message.into(),
        details: details.into(),
    };
    tauri_plugin_log::log::error!(
        target: "backend",
        "{}: {} | {}",
        event.kind,
        event.message,
        event.details
    );
    if let Err(error) = app.emit("backend-error", &event) {
        tauri_plugin_log::log::error!(target: "backend", "Could not emit backend error: {error}");
    }
}

/// All errors to do with the db
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// sqlx err
    #[error("data store disconnected")]
    Sqlx(sqlx::Error),
    #[error("A backend task failed")]
    Tauri(tauri::Error),
    #[error("{0}")]
    Unknown(String),
}

impl Drop for DatabaseError {
    fn drop(&mut self) {
        if let Some(app) = APP_HANDLE.get() {
            report_backend_error(app, "DatabaseError", self.to_string(), format!("{self:?}"));
        }
    }
}

pub fn report_registered_backend_error(
    kind: impl Into<String>,
    message: impl Into<String>,
    details: impl Into<String>,
) {
    if let Some(app) = APP_HANDLE.get() {
        report_backend_error(app, kind, message, details);
    }
}
