use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct StartupError {
    pub message: String,
    pub details: String,
}

impl StartupError {
    pub fn new(message: &str, error: impl std::fmt::Display) -> Self {
        Self {
            message: message.into(),
            details: error.to_string(),
        }
    }
}

pub struct StartupStatus {
    pub error: Option<StartupError>,
}

#[tauri::command]
pub fn get_startup_error(status: tauri::State<'_, StartupStatus>) -> Option<StartupError> {
    status.error.clone()
}

#[tauri::command]
pub fn relaunch_app(app: tauri::AppHandle) {
    app.request_restart();
}
