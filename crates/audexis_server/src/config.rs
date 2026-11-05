use crate::{error::StartupResult, storage::Storage, users};
use std::{net::IpAddr, path::PathBuf};

pub(crate) struct ServerConfig {
    pub bind: String,
    pub storage: Storage,
    pub auth_origin: Option<String>,
    pub default_quota: i64,
    pub max_file: i64,
    pub trusted_proxies: Vec<IpAddr>,
    pub ingest_dir: Option<PathBuf>,
    pub ffmpeg: Option<String>,
}

impl ServerConfig {
    pub fn from_env() -> StartupResult<Self> {
        let mode = std::env::var("AUDEXIS_UPLOAD_MODE").unwrap_or_else(|_| "server".into());
        if !matches!(mode.as_str(), "server" | "direct") {
            return Err("AUDEXIS_UPLOAD_MODE must be server or direct".into());
        }
        let ingest_dir = if mode == "server" {
            let origin = std::env::var("AUDEXIS_PUBLIC_ORIGIN").map_err(|_| "Set AUDEXIS_PUBLIC_ORIGIN to the externally reachable server URL for server uploads")?;
            let url = reqwest::Url::parse(&origin)?;
            if !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err("Invalid AUDEXIS_PUBLIC_ORIGIN".into());
            }
            Some(
                std::env::var("AUDEXIS_INGEST_DIR")
                    .unwrap_or_else(|_| "./data/ingest".into())
                    .into(),
            )
        } else {
            None
        };
        Ok(Self {
            bind: std::env::var("AUDEXIS_BIND").unwrap_or_else(|_| "127.0.0.1:3000".into()),
            storage: Storage::from_env().map_err(|error| error.1)?,
            auth_origin: std::env::var("AUDEXIS_AUTH_ORIGIN")
                .ok()
                .map(|value| value.trim().trim_end_matches('/').to_owned())
                .filter(|value| !value.is_empty()),
            default_quota: users::default_quota()?,
            max_file: std::env::var("AUDEXIS_MAX_FILE_BYTES")
                .unwrap_or_else(|_| "1073741824".into())
                .parse()?,
            trusted_proxies: std::env::var("AUDEXIS_TRUSTED_PROXIES")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::parse)
                .collect::<std::result::Result<Vec<IpAddr>, _>>()?,
            ingest_dir,
            ffmpeg: std::env::var("AUDEXIS_FFMPEG_PATH")
                .ok()
                .filter(|value| !value.is_empty()),
        })
    }
}

pub(crate) fn load_env() -> StartupResult<()> {
    let loaded = dotenvy::from_path("crates/audexis_server/.env").or_else(|error| {
        if error.not_found() {
            dotenvy::dotenv().map(|_| ())
        } else {
            Err(error)
        }
    });
    if let Err(error) = loaded {
        if !error.not_found() {
            return Err(std::io::Error::other("Could not load server .env").into());
        }
    }
    Ok(())
}
