use crate::utils::tasks::{self, TaskProgress};

#[tauri::command]
pub fn get_active_tasks() -> Vec<TaskProgress> {
    tasks::active()
}
