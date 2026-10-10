use tauri::{command, AppHandle};

use crate::AppState;

use super::shared::finish_playback_and_clear_queue;

#[command]
pub fn stop_playback(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    finish_playback_and_clear_queue(&app, &state)
}
