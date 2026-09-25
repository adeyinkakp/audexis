use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Executor;
use std::fs;
use std::path::PathBuf;
pub mod types;
#[derive(Clone, Debug)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn init(db_path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let config_folder = db_path.parent();
        if config_folder.is_none() {
            panic!("No Config folder bruh")
        }
        let config_folder = config_folder.unwrap();
        println!("{}", &config_folder.as_os_str().display());
        fs::create_dir_all(config_folder).expect("Could not create config folder");

        let connection_options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(db_path)
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
