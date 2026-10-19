use log::LevelFilter;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{ConnectOptions, Executor};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

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
            .log_statements(LevelFilter::Trace)
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

        sqlx::migrate!("src/database/migrations").run(&pool).await?;
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
