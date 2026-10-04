use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{ConnectOptions, Executor};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri_plugin_log::log::LevelFilter;
pub mod types;
#[derive(Clone, Debug)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn init(db_path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let config_folder = db_path.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Database path has no parent folder",
            )
        })?;
        fs::create_dir_all(config_folder)?;

        let connection_options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(db_path)
            .log_statements(LevelFilter::Debug)
            .log_slow_statements(LevelFilter::Warn, Duration::from_millis(50))
            .collation("METADATA_NOCASE", |a, b| {
                a.trim().to_lowercase().cmp(&b.trim().to_lowercase())
            });

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .after_connect(|conn, _| {
                Box::pin(async move {
                    conn.execute("PRAGMA journal_mode=WAL;").await?;
                    Ok(())
                })
            })
            .connect_with(connection_options)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;
        println!("migrations ran");
        Ok(Self { pool })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_data_folder_returns_error_without_panicking() {
        let path = std::env::temp_dir().join(format!("audexis-startup-{}", uuid::Uuid::new_v4()));
        fs::write(&path, b"not a directory").unwrap();
        let result = tauri::async_runtime::block_on(Database::init(&path.join("library.db")));
        fs::remove_file(&path).unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn database_path_without_parent_returns_error_without_panicking() {
        let path = PathBuf::new();
        assert!(tauri::async_runtime::block_on(Database::init(&path)).is_err());
    }
}
