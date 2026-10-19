use super::*;

/// Generates a temporary file path for a given target file.
pub fn temp_path_for(target: &Path) -> PathBuf {
    let mut p = target.to_path_buf();
    let file_name = target
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    p.set_file_name(format!(".{}.tmp", file_name));
    p
}

/// Replaces the target file with the temporary file.
pub fn replace_tmp(tmp: &Path, target: &Path) -> Result<(), BackendError> {
    fs::rename(tmp, target).map_err(|error| {
        BackendError::WriteFailed(TagError {
            path: target.to_string_lossy().into_owned(),
            public_message: "Could not replace file".into(),
            internal_message: error.to_string(),
        })
    })
}
