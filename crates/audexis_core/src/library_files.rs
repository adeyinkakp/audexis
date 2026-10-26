use std::{
    fs::File,
    io,
    path::Path,
    sync::{Arc, Mutex, Weak},
};
use tokio::{runtime::Handle, sync::broadcast};

static REPORTER: Mutex<Option<Weak<LibraryFiles>>> = Mutex::new(None);

#[derive(Clone, Debug)]
pub struct LibraryChange {
    pub file_ids: Vec<i64>,
    pub artwork_only: bool,
}

pub struct LibraryFiles {
    pool: sqlx::SqlitePool,
    runtime: Handle,
    changes: broadcast::Sender<LibraryChange>,
    on_reconcile: Box<dyn Fn(std::collections::HashMap<i64, String>, Vec<i64>) + Send + Sync>,
}
impl LibraryFiles {
    pub fn new(
        pool: sqlx::SqlitePool,
        runtime: Handle,
        on_reconcile: impl Fn(std::collections::HashMap<i64, String>, Vec<i64>) + Send + Sync + 'static,
    ) -> Arc<Self> {
        let (changes, _) = broadcast::channel(64);
        Arc::new(Self {
            pool,
            runtime,
            changes,
            on_reconcile: Box::new(on_reconcile),
        })
    }
    pub fn reconcile(&self, paths: std::collections::HashMap<i64, String>, missing: Vec<i64>) {
        if !paths.is_empty() || !missing.is_empty() {
            (self.on_reconcile)(paths, missing);
        }
    }
    pub fn publish_changed(&self, file_ids: Vec<i64>) {
        let _ = self.changes.send(LibraryChange {
            file_ids,
            artwork_only: false,
        });
    }
    pub fn publish_artwork_changed(&self, file_ids: Vec<i64>) {
        let _ = self.changes.send(LibraryChange {
            file_ids,
            artwork_only: true,
        });
    }
    pub fn subscribe(&self) -> broadcast::Receiver<LibraryChange> {
        self.changes.subscribe()
    }
    fn report(self: &Arc<Self>, path: &Path) {
        let reporter = self.clone();
        let path = path.to_owned();
        self.runtime.spawn(async move {
            let ids = match mark_missing(&reporter.pool, &path).await {
                Ok(ids) if ids.is_empty() => return,
                Ok(ids) => ids,
                Err(error) => {
                    tracing::error!(path = %path.display(), %error, "Could not mark file missing");
                    return;
                }
            };
            reporter.reconcile(Default::default(), ids.clone());
            let _ = reporter.changes.send(LibraryChange {
                file_ids: ids,
                artwork_only: false,
            });
        });
    }
}

pub fn init(reporter: &Arc<LibraryFiles>) {
    *REPORTER.lock().unwrap() = Some(Arc::downgrade(reporter));
}
pub fn is_virtual_path(path: &Path) -> bool {
    path.to_str().is_some_and(|path| {
        ["cloud-library://", "cloud-stream://", "http://", "https://"]
            .iter()
            .any(|prefix| path.starts_with(prefix))
    })
}
fn require_disk_path(path: &Path) -> io::Result<()> {
    if is_virtual_path(path) {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cloud playback handles cannot be opened as disk files",
        ))
    } else {
        Ok(())
    }
}
pub fn observe<T>(path: &Path, result: io::Result<T>) -> io::Result<T> {
    if let Err(error) = &result
        && !is_virtual_path(path)
        && error.kind() == io::ErrorKind::NotFound
    {
        let reporter = REPORTER.lock().unwrap().as_ref().and_then(Weak::upgrade);
        if let Some(reporter) = reporter {
            reporter.report(path);
        }
    }
    result
}
pub fn open(path: impl AsRef<Path>) -> io::Result<File> {
    let path = path.as_ref();
    require_disk_path(path)?;
    observe(path, File::open(path))
}
pub fn open_for_update(path: impl AsRef<Path>) -> io::Result<File> {
    let path = path.as_ref();
    require_disk_path(path)?;
    observe(
        path,
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path),
    )
}
pub fn metadata(path: impl AsRef<Path>) -> io::Result<std::fs::Metadata> {
    let path = path.as_ref();
    require_disk_path(path)?;
    observe(path, std::fs::metadata(path))
}
pub fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let path = path.as_ref();
    require_disk_path(path)?;
    observe(path, std::fs::read(path))
}
pub fn check_after_error(path: &Path) {
    let _ = metadata(path);
}

pub(crate) async fn mark_missing(
    pool: &sqlx::SqlitePool,
    path: &Path,
) -> Result<Vec<i64>, sqlx::Error> {
    if is_virtual_path(path) {
        return Ok(Vec::new());
    }
    sqlx::query_scalar("UPDATE files SET missing_since = unixepoch() WHERE path = ?1 AND cloud_connection_id IS NULL AND missing_since IS NULL RETURNING id")
        .bind(path.to_string_lossy().as_ref()).fetch_all(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_opens_existing_files_without_truncating_or_creating() {
        use std::io::Write;
        let path = std::env::temp_dir().join(format!("audexis-file-{}", uuid::Uuid::new_v4()));
        assert_eq!(
            open_for_update(&path).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert!(!path.exists());
        std::fs::write(&path, b"abc").unwrap();
        let mut file = open_for_update(&path).unwrap();
        file.write_all(b"z").unwrap();
        drop(file);
        assert_eq!(read(&path).unwrap(), b"zbc");
        assert_eq!(metadata(&path).unwrap().len(), 3);
        std::fs::remove_file(path).unwrap();
    }
    #[tokio::test]
    async fn missing_open_marks_database_and_notifies_once() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT, missing_since INTEGER, cloud_connection_id TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let path = std::env::temp_dir().join(format!("audexis-missing-{}", uuid::Uuid::new_v4()));
        sqlx::query("INSERT INTO files VALUES (1, ?, NULL, NULL)")
            .bind(path.to_string_lossy().as_ref())
            .execute(&pool)
            .await
            .unwrap();
        let reconciled = Arc::new(Mutex::new(Vec::new()));
        let result = reconciled.clone();
        let reporter = LibraryFiles::new(pool.clone(), Handle::current(), move |_, ids| {
            result.lock().unwrap().extend_from_slice(&ids)
        });
        init(&reporter);
        let mut events = reporter.subscribe();
        let denied: io::Result<()> = observe(&path, Err(io::ErrorKind::PermissionDenied.into()));
        assert_eq!(denied.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
        assert!(events.try_recv().is_err());
        assert_eq!(open(&path).unwrap_err().kind(), io::ErrorKind::NotFound);
        let change = tokio::time::timeout(std::time::Duration::from_secs(2), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(change.file_ids, vec![1]);
        assert_eq!(*reconciled.lock().unwrap(), vec![1]);
        assert!(mark_missing(&pool, &path).await.unwrap().is_empty());
        assert!(
            mark_missing(&pool, Path::new("/unindexed"))
                .await
                .unwrap()
                .is_empty()
        );
        drop(reporter);
        assert_eq!(read(&path).unwrap_err().kind(), io::ErrorKind::NotFound);
    }
}
