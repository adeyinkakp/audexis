mod backfill;
mod classification;
mod events;
mod metadata;
mod paths;
mod reconciliation;
mod scan;

use std::path::Path;
use std::time::Duration;

use notify::{RecommendedWatcher, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use tauri::{async_runtime, AppHandle, Manager};

use crate::database::Database;
use crate::utils::database::get_library_roots;
use crate::utils::errors::DatabaseError;
use crate::AppState;

pub(crate) use paths::is_audio_file;

#[derive(Debug)]
pub struct FileWatcher {
    debouncer: Debouncer<RecommendedWatcher, FileIdMap>,

    app_handle: AppHandle,
}

impl FileWatcher {
    pub fn new(app: &AppHandle) -> Result<Self, DatabaseError> {
        let app_handle = app.clone();

        let debouncer = new_debouncer(
            Duration::from_millis(500),
            None,
            move |res: Result<Vec<DebouncedEvent>, _>| match res {
                Ok(events) => {
                    FileWatcher::handle_events(&app_handle, &events);
                }
                Err(error) => crate::utils::errors::report_backend_error(
                    &app_handle,
                    "FileWatcherError",
                    "The library watcher encountered an error",
                    format!("{error:?}"),
                ),
            },
        )
        .map_err(|error| DatabaseError::Unknown(error.to_string()))?;
        let new_watcher = Self {
            app_handle: app.clone(),
            debouncer,
        };

        Ok(new_watcher)
    }
    pub async fn init(&mut self) -> Result<(), DatabaseError> {
        let app_handle = self.app_handle.clone();
        let state: tauri::State<'_, AppState> = app_handle.state::<AppState>();
        let db: &Database = &state.db;
        let roots = get_library_roots(&db.pool).await?;

        for root in &roots {
            let pth = Path::new(&root);
            if !pth.exists() {
                continue;
            }
            if pth.is_file() {
                continue;
            }
            if let Err(error) = self
                .debouncer
                .watcher()
                .watch(pth, notify::RecursiveMode::Recursive)
            {
                crate::utils::errors::report_backend_error(
                    &self.app_handle,
                    "FileWatcherError",
                    "Could not watch a library folder",
                    format!("{root}: {error}"),
                );
            }
        }

        let scan_app_handle = self.app_handle.clone();
        async_runtime::spawn(async move {
            if let Err(error) = FileWatcher::scan_folders_for_app(&scan_app_handle, roots).await {
                drop(error);
            }
        });

        Ok(())
    }
    pub fn unwatch_folders(&mut self, folders: Vec<String>) {
        for folder in folders {
            let _ = self.debouncer.watcher().unwatch(Path::new(&folder));
        }
    }

    pub fn watch_folders(&mut self, folders: Vec<String>) {
        for root in folders {
            let pth = Path::new(&root);
            if !pth.exists() {
                continue;
            }
            if pth.is_file() {
                continue;
            }

            let _ = self
                .debouncer
                .watcher()
                .watch(pth, notify::RecursiveMode::Recursive);
        }
    }
}
