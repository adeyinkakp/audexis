use crate::{
    auth,
    config::ServerConfig,
    error::{Error, Result, StartupResult},
    storage::Storage,
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, Notify};

pub(crate) struct Ticket {
    pub(crate) key: String,
    pub(crate) write: bool,
    pub(crate) size: i64,
    pub(crate) expires: Instant,
    pub(crate) expected_hash: Option<String>,
}
pub struct AppState {
    pub(crate) db: sqlx::PgPool,
    pub(crate) storage: Storage,
    pub(crate) client: reqwest::Client,
    pub(crate) auth_origin: Option<String>,
    pub(crate) default_quota: i64,
    pub(crate) max_file: i64,
    pub(crate) limits: auth::Limits,
    pub(crate) trusted_proxies: Vec<std::net::IpAddr>,
    pub(crate) processing: Notify,
    pub(crate) tickets: Mutex<HashMap<String, Ticket>>,
    pub(crate) ingest_dir: Option<std::path::PathBuf>,
    pub(crate) ffmpeg: Option<String>,
}
impl AppState {
    pub(crate) async fn new(db: sqlx::PgPool, config: ServerConfig) -> StartupResult<Self> {
        let has_users: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM self_hosted_users)")
            .fetch_one(&db)
            .await?;
        if config.auth_origin.is_none() && !has_users {
            return Err("No users configured. Run audexis_server users create <name>, or configure AUDEXIS_AUTH_ORIGIN.".into());
        }
        Ok(Self {
            db,
            storage: config.storage,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(300))
                .connect_timeout(Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            auth_origin: config.auth_origin,
            default_quota: config.default_quota,
            max_file: config.max_file,
            limits: auth::Limits::default(),
            trusted_proxies: config.trusted_proxies,
            processing: Notify::new(),
            tickets: Mutex::new(HashMap::new()),
            ingest_dir: config.ingest_dir,
            ffmpeg: config.ffmpeg,
        })
    }
    pub(crate) async fn revoke_upload_tickets(&self, key: &str) {
        self.tickets
            .lock()
            .await
            .retain(|_, ticket| !ticket.write || ticket.key != key);
    }
    pub(crate) fn stage_path(&self, key: &str) -> Result<std::path::PathBuf> {
        if let Some(root) = &self.ingest_dir {
            if !key.starts_with("staging/")
                || !key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"/-_".contains(&b))
            {
                return Err(Error::bad("Invalid upload staging key"));
            }
            Ok(root.join(key))
        } else {
            self.storage.path(key)
        }
    }
    pub(crate) async fn delete_stage(&self, key: &str) -> Result<()> {
        if let Ok(path) = self.stage_path(key) {
            match tokio::fs::remove_file(path).await {
                Ok(()) => return Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        self.storage.delete(&self.client, key).await
    }
    pub(crate) async fn ticket(&self, key: String, write: bool, size: i64) -> String {
        let mut tickets = self.tickets.lock().await;
        tickets.retain(|_, v| v.expires > Instant::now());
        if tickets.len() >= 10000 {
            if let Some(oldest) = tickets
                .iter()
                .min_by_key(|(_, t)| t.expires)
                .map(|(k, _)| k.clone())
            {
                tickets.remove(&oldest);
            }
        }
        let token = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        tickets.insert(
            token.clone(),
            Ticket {
                key,
                write,
                size,
                expires: Instant::now() + Duration::from_secs(900),
                expected_hash: None,
            },
        );
        token
    }
}
