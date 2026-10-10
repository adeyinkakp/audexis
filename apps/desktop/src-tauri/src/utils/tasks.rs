use once_cell::sync::Lazy;
use serde::Serialize;
use std::{collections::HashMap, sync::Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    pub id: String,
    pub label: String,
    pub status: String,
    pub current: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
}

static TASKS: Lazy<Mutex<HashMap<String, TaskProgress>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn emit(app: &AppHandle, task: &TaskProgress) {
    let _ = app.emit("task-progress", task);
}

pub struct TaskGuard {
    app: AppHandle,
    id: String,
    finished: bool,
}

impl TaskGuard {
    pub fn start(app: &AppHandle, id: &str, label: &str, total: Option<u64>) -> Self {
        let task = TaskProgress {
            id: id.to_string(),
            label: label.to_string(),
            status: "running".to_string(),
            current: 0,
            total,
            message: None,
        };
        if let Ok(mut tasks) = TASKS.lock() {
            tasks.insert(task.id.clone(), task.clone());
        }
        emit(app, &task);
        Self {
            app: app.clone(),
            id: id.to_string(),
            finished: false,
        }
    }

    pub fn update(&self, current: u64, total: Option<u64>, message: Option<String>) {
        update(&self.app, &self.id, current, total, message);
    }

    pub fn complete(mut self, message: impl Into<String>) {
        finish(&self.app, &self.id, "completed", message.into());
        self.finished = true;
    }
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        if !self.finished {
            finish(
                &self.app,
                &self.id,
                "failed",
                "Task stopped before completion".to_string(),
            );
        }
    }
}

pub fn update(
    app: &AppHandle,
    id: &str,
    current: u64,
    total: Option<u64>,
    message: Option<String>,
) {
    if let Ok(mut tasks) = TASKS.lock() {
        if let Some(task) = tasks.get_mut(id) {
            task.current = current;
            task.total = total;
            task.message = message;
            emit(app, task);
        }
    }
}

fn finish(app: &AppHandle, id: &str, status: &str, message: String) {
    let task = TASKS.lock().ok().and_then(|mut tasks| tasks.remove(id));
    if let Some(mut task) = task {
        task.status = status.to_string();
        task.message = Some(message);
        if let Some(total) = task.total {
            task.current = total;
        }
        emit(app, &task);
    }
}

pub fn active() -> Vec<TaskProgress> {
    TASKS
        .lock()
        .map(|tasks| tasks.values().cloned().collect())
        .unwrap_or_default()
}
