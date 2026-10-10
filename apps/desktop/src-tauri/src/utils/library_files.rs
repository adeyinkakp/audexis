use std::{fs::File, io, path::Path};
use tauri::{AppHandle, Emitter, Manager};

static APP: once_cell::sync::OnceCell<AppHandle> = once_cell::sync::OnceCell::new();

pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

pub fn observe<T>(path: &Path, result: io::Result<T>) -> io::Result<T> {
    if let Err(error) = &result {
        if error.kind() == io::ErrorKind::NotFound {
            report_missing(path);
        }
    }
    result
}

pub fn open(path: impl AsRef<Path>) -> io::Result<File> {
    let path = path.as_ref();
    observe(path, File::open(path))
}

pub fn open_for_update(path: impl AsRef<Path>) -> io::Result<File> {
    let path = path.as_ref();
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
    observe(path, std::fs::metadata(path))
}

pub fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let path = path.as_ref();
    observe(path, std::fs::read(path))
}

pub fn check_after_error(path: &Path) {
    let _ = metadata(path);
}

pub(crate) async fn mark_missing(
    pool: &sqlx::SqlitePool,
    path: &Path,
) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar(
        "UPDATE files SET missing_since = unixepoch()
         WHERE path = ?1 AND missing_since IS NULL RETURNING id",
    )
    .bind(path.to_string_lossy().as_ref())
    .fetch_all(pool)
    .await
}

fn report_missing(path: &Path) {
    let Some(app) = APP.get().cloned() else {
        return;
    };
    let path = path.to_owned();

    tauri::async_runtime::spawn(async move {
        let Some(state) = app.try_state::<crate::AppState>() else {
            return;
        };
        let ids = match mark_missing(&state.db.pool, &path).await {
            Ok(ids) if ids.is_empty() => return,
            Ok(ids) => ids,
            Err(error) => {
                tauri_plugin_log::log::error!("Could not mark {} missing: {error}", path.display());
                return;
            }
        };
        #[derive(Clone, serde::Serialize)]
        struct LibraryChange {
            file_ids: Vec<i64>,
        }
        if let Err(error) = app.emit(
            "library-changed",
            LibraryChange {
                file_ids: ids.clone(),
            },
        ) {
            tauri_plugin_log::log::error!("Could not publish missing files: {error}");
        }
        if let Err(error) = crate::commands::playback::shared::reconcile_queue_with_library(
            &app,
            &state,
            Default::default(),
            ids.into_iter().collect(),
        )
        .await
        {
            tauri_plugin_log::log::error!("Could not reconcile missing files: {error}");
        }
    });
}
