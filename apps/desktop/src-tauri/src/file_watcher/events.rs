use crate::utils::errors::DatabaseError;
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
struct LibraryChange<'a> {
    file_ids: &'a [i64],
}

pub(super) fn emit_changed(app: &AppHandle, ids: &[i64]) -> Result<(), DatabaseError> {
    for batch in ids.chunks(200) {
        app.emit("library-changed", LibraryChange { file_ids: batch })
            .map_err(|error| DatabaseError::Unknown(error.to_string()))?;
    }
    Ok(())
}
