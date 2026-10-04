use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::utils::errors::DatabaseError;

fn indexed_path_prefix(path: &Path) -> String {
    let mut prefix = path
        .to_string_lossy()
        .trim_end_matches(std::path::MAIN_SEPARATOR)
        .to_string();
    prefix.push(std::path::MAIN_SEPARATOR);
    prefix
}

pub(super) fn is_in_library(path: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| path.starts_with(root))
}

pub(super) async fn mark_indexed_path_missing(
    pool: &sqlx::SqlitePool,
    path: &Path,
) -> Result<Vec<i64>, DatabaseError> {
    let path = path.to_string_lossy().to_string();
    let prefix = indexed_path_prefix(Path::new(&path));
    sqlx::query_scalar::<_, i64>(
        "UPDATE files SET missing_since = unixepoch()
         WHERE (path = ?1 OR substr(path, 1, length(?2)) = ?2)
           AND missing_since IS NULL
         RETURNING id",
    )
    .bind(path)
    .bind(prefix)
    .fetch_all(pool)
    .await
    .map_err(DatabaseError::Sqlx)
}

pub(super) async fn rebase_indexed_directory(
    pool: &sqlx::SqlitePool,
    old_path: &Path,
    new_path: &Path,
) -> Result<Vec<i64>, DatabaseError> {
    let old_path_string = old_path.to_string_lossy().to_string();
    let old_prefix = indexed_path_prefix(old_path);
    let indexed_files = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, path FROM files
         WHERE path = ?1 OR substr(path, 1, length(?2)) = ?2",
    )
    .bind(old_path_string)
    .bind(old_prefix)
    .fetch_all(pool)
    .await
    .map_err(DatabaseError::Sqlx)?;

    let mut changed_ids = Vec::with_capacity(indexed_files.len());
    for (id, source) in indexed_files {
        let Ok(suffix) = Path::new(&source).strip_prefix(old_path) else {
            continue;
        };
        let destination = new_path.join(suffix);
        let Ok(metadata) = std::fs::metadata(&destination) else {
            sqlx::query("DELETE FROM files WHERE id = ?1")
                .bind(id)
                .execute(pool)
                .await
                .map_err(DatabaseError::Sqlx)?;
            changed_ids.push(id);
            continue;
        };
        if !metadata.is_file() || !is_audio_file(&destination) {
            continue;
        }

        let destination_string = destination.to_string_lossy().to_string();
        let replaced_ids = sqlx::query_scalar::<_, i64>(
            "DELETE FROM files WHERE path = ?1 AND id != ?2 RETURNING id",
        )
        .bind(&destination_string)
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(DatabaseError::Sqlx)?;
        changed_ids.extend(replaced_ids);
        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or_default();
        let file_name = destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file");
        sqlx::query(
            "UPDATE files
             SET path = ?1, file_name = ?2, size = ?3, modified_at = ?4,
                 status = 'pending', metadata_status = 'ok', missing_since = NULL
             WHERE id = ?5",
        )
        .bind(destination_string)
        .bind(file_name)
        .bind(metadata.len() as i64)
        .bind(modified_at)
        .bind(id)
        .execute(pool)
        .await
        .map_err(DatabaseError::Sqlx)?;
        changed_ids.push(id);
    }
    Ok(changed_ids)
}

pub(crate) fn is_audio_file(p: &Path) -> bool {
    const SUPPORTED_EXTENSIONS: [&str; 15] = [
        "m4a", "mp4", "qt", "m4b", "m4v", "mov", "ogg", "opus", "oga", "spx", "ogv", "mp3", "mp2",
        "mp1", "flac",
    ];
    p.extension().is_some_and(|v| {
        v.to_str()
            .is_some_and(|s| SUPPORTED_EXTENSIONS.contains(&s))
    })
}
