use tauri::command;

use crate::AppState;

use super::shared::{get_queue_info, QueueInfo};

#[command]
pub fn get_queue(state: tauri::State<'_, AppState>) -> Result<QueueInfo, String> {
    get_queue_info(&state)
}
