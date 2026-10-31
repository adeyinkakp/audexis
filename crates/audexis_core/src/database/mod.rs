use log::LevelFilter;
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

mod connections;
pub mod discovery;
pub(crate) mod search;
pub mod songs;
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
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(15))
            .log_statements(LevelFilter::Trace)
            .log_slow_statements(LevelFilter::Warn, Duration::from_millis(50))
            .collation("METADATA_NOCASE", |a, b| {
                a.trim().to_lowercase().cmp(&b.trim().to_lowercase())
            });

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connection_options)
            .await?;

        sqlx::migrate!("src/database/migrations").run(&pool).await?;
        Database { pool: pool.clone() }
            .set_cloud_library_connection(None)
            .await?;
        search::refresh_pending(&pool).await?;
        println!("migrations ran");
        Ok(Self { pool })
    }
}

mod library;

pub mod collections;
pub mod media_actions;
pub mod missing;
pub mod rewind;
