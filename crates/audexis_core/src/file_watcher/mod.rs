mod backfill;
mod classification;
mod events;
mod hashing;
mod metadata;
mod paths;
mod reconciliation;
mod scan;

use crate::{database::Database, library_files::LibraryFiles};
pub use events::WatcherStatus;
use notify::{RecommendedWatcher, Watcher};
use notify_debouncer_full::{DebouncedEvent, Debouncer, FileIdMap, new_debouncer};
pub(crate) use paths::is_audio_file;
use std::{
    path::Path,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Duration,
};
use tokio::sync::{mpsc, watch};

#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("{0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("{0}")]
    Unknown(String),
}
#[derive(Clone)]
pub(super) struct WatcherContext {
    db: Database,
    files: Arc<LibraryFiles>,
    library_scan_lock: Arc<tokio::sync::Mutex<()>>,
    pending_worker_running: Arc<AtomicBool>,
    status: watch::Sender<WatcherStatus>,
}
enum Request {
    Scan,
    Events(Vec<DebouncedEvent>),
}

pub struct FileWatcher {
    debouncer: Mutex<Debouncer<RecommendedWatcher, FileIdMap>>,
    context: WatcherContext,
    requests: mpsc::UnboundedSender<Request>,
    worker: tokio::task::JoinHandle<()>,
}
impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.worker.abort();
    }
}
impl FileWatcher {
    pub async fn start(db: Database, files: Arc<LibraryFiles>) -> Result<Self, WatcherError> {
        let (status, _) = watch::channel(WatcherStatus::default());
        let context = WatcherContext {
            db,
            files,
            library_scan_lock: Arc::default(),
            pending_worker_running: Arc::default(),
            status,
        };
        let (requests, mut receiver) = mpsc::unbounded_channel();
        let sender = requests.clone();
        let errors = context.status.clone();
        let mut debouncer = new_debouncer(
            Duration::from_millis(500),
            None,
            move |result: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| match result {
                Ok(events) => {
                    let _ = sender.send(Request::Events(events));
                }
                Err(error) => {
                    let message = format!("Library watcher: {error:?}");
                    tracing::error!(%message);
                    errors.send_modify(|s| s.error = Some(message));
                }
            },
        )
        .map_err(|e| WatcherError::Unknown(e.to_string()))?;
        for root in get_library_roots(&context.db.pool).await? {
            let path = Path::new(&root);
            if path.is_dir() {
                debouncer
                    .cache()
                    .add_root(path, notify::RecursiveMode::Recursive);
                if let Err(error) = debouncer
                    .watcher()
                    .watch(path, notify::RecursiveMode::Recursive)
                {
                    context.status.send_modify(|s| {
                        s.error = Some(format!("Could not watch {root}: {error}"))
                    });
                }
            }
        }
        requests.send(Request::Scan).unwrap();
        let worker_context = context.clone();
        let worker = tokio::spawn(async move {
            while let Some(request) = receiver.recv().await {
                let result = match request {
                    Request::Scan => Self::scan_folders_for_context(&worker_context).await,
                    Request::Events(events) => Self::handle_events(&worker_context, &events).await,
                };
                if let Err(error) = result {
                    tracing::error!(%error, "Library watcher failed");
                    worker_context.status.send_modify(|s| {
                        s.busy = false;
                        s.error = Some(error.to_string());
                    });
                }
            }
        });
        Ok(Self {
            debouncer: Mutex::new(debouncer),
            context,
            requests,
            worker,
        })
    }
    pub fn subscribe(&self) -> watch::Receiver<WatcherStatus> {
        self.context.status.subscribe()
    }
    pub fn rescan(&self) -> Result<(), WatcherError> {
        self.requests
            .send(Request::Scan)
            .map_err(|_| WatcherError::Unknown("Watcher has stopped".into()))
    }
    pub async fn add_folder(&self, path: &Path) -> Result<(), WatcherError> {
        let path = tokio::fs::canonicalize(path)
            .await
            .map_err(|e| WatcherError::Unknown(e.to_string()))?;
        if !path.is_dir() {
            return Err(WatcherError::Unknown("Choose a folder".into()));
        }
        {
            let mut debouncer = self.debouncer.lock().unwrap();
            debouncer
                .watcher()
                .watch(&path, notify::RecursiveMode::Recursive)
                .map_err(|e| WatcherError::Unknown(e.to_string()))?;
            debouncer
                .cache()
                .add_root(&path, notify::RecursiveMode::Recursive);
        }
        sqlx::query("INSERT INTO import_roots(path) VALUES (?) ON CONFLICT(path) DO NOTHING")
            .bind(path.to_string_lossy().as_ref())
            .execute(&self.context.db.pool)
            .await?;
        self.context.files.publish_changed(Vec::new());
        self.rescan()
    }

    pub async fn remove_folder(&self, path: &Path) -> Result<(), WatcherError> {
        let path = tokio::fs::canonicalize(path)
            .await
            .map_err(|e| WatcherError::Unknown(e.to_string()))?;
        {
            let mut debouncer = self
                .debouncer
                .lock()
                .map_err(|_| WatcherError::Unknown("Watcher lock poisoned".into()))?;
            debouncer
                .watcher()
                .unwatch(&path)
                .map_err(|e| WatcherError::Unknown(e.to_string()))?;
        }
        remove_library_folder(&self.context.db.pool, &path).await?;
        self.context.files.publish_changed(Vec::new());
        self.rescan()
    }
}

async fn remove_library_folder(pool: &sqlx::SqlitePool, path: &Path) -> Result<(), WatcherError> {
    let path = path.to_string_lossy().to_string();
    let prefix = format!("{}/", path.trim_end_matches('/'));
    let mut transaction = pool.begin().await?;

    sqlx::query("DELETE FROM import_roots WHERE path = ?")
        .bind(&path)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "DELETE FROM files
         WHERE path = ? OR substr(path, 1, length(?)) = ?",
    )
    .bind(&path)
    .bind(&prefix)
    .bind(&prefix)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;
    Ok(())
}
pub async fn get_library_roots(pool: &sqlx::SqlitePool) -> Result<Vec<String>, WatcherError> {
    Ok(
        sqlx::query_scalar("SELECT path FROM import_roots ORDER BY path")
            .fetch_all(pool)
            .await?,
    )
}

#[cfg(test)]
mod tests;
