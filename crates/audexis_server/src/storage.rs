mod signing;
use signing::*;

mod response;
use response::*;

use crate::error::{Error, Result};
use axum::http::StatusCode;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicI64, Ordering};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
static CAP_UNTIL: AtomicI64 = AtomicI64::new(0);
pub fn cooldown_seconds() -> u64 {
    CAP_UNTIL
        .load(Ordering::Relaxed)
        .saturating_sub(Utc::now().timestamp())
        .max(0) as u64
}
pub fn cooldown_error() -> Option<Error> {
    (cooldown_seconds() > 0).then(|| {
        Error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Storage daily limit reached. Cloud transfers will retry after a cooldown".into(),
        )
    })
}
fn record_cap() -> Error {
    let now = Utc::now().timestamp();
    CAP_UNTIL.store(
        (now + 900).min((now.div_euclid(86400) + 1) * 86400 + 60),
        Ordering::Relaxed,
    );
    cooldown_error().unwrap()
}

pub enum Storage {
    B2 {
        endpoint: String,
        region: String,
        bucket: String,
        key_id: String,
        secret: String,
    },
    Local {
        root: PathBuf,
        origin: String,
    },
}
impl Storage {
    pub fn from_env() -> Result<Self> {
        if let Ok(root) = std::env::var("AUDEXIS_STORAGE_DIR") {
            return Ok(Self::Local {
                root: root.into(),
                origin: env("AUDEXIS_PUBLIC_ORIGIN")?.trim_end_matches('/').into(),
            });
        }
        let endpoint = env("B2_ENDPOINT")?;
        let url = reqwest::Url::parse(&endpoint).map_err(|_| Error::bad("Invalid B2 endpoint"))?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
        {
            return Err(Error::bad(
                "B2 endpoint must be HTTPS without credentials or a query",
            ));
        }
        let region = env("B2_REGION")?;
        validate_region(&url, &region)?;
        Ok(Self::B2 {
            endpoint: endpoint.trim_end_matches('/').into(),
            region,
            bucket: env("B2_BUCKET")?,
            key_id: env("B2_KEY_ID")?,
            secret: env("B2_APPLICATION_KEY")?,
        })
    }
    pub fn signed(&self, method: &str, key: &str, length: Option<i64>) -> Result<String> {
        self.signed_query(method, key, length, &[])
    }
    fn signed_query(
        &self,
        method: &str,
        key: &str,
        length: Option<i64>,
        extra: &[(&str, String)],
    ) -> Result<String> {
        if let Some(error) = cooldown_error() {
            return Err(error);
        }
        let Self::B2 {
            endpoint,
            region,
            bucket,
            key_id,
            secret,
        } = self
        else {
            return Err(Error::bad("Local storage uses server transfer tickets"));
        };
        sign(
            endpoint,
            region,
            bucket,
            key_id,
            secret,
            method,
            key,
            length,
            &Utc::now().format("%Y%m%dT%H%M%SZ").to_string(),
            900,
            extra,
        )
    }
    pub fn path(&self, key: &str) -> Result<PathBuf> {
        if key.split('/').any(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        }) {
            return Err(Error::bad("Invalid object key"));
        }
        match self {
            Self::Local { root, .. } => Ok(root.join(key)),
            _ => Err(Error::bad("Not local storage")),
        }
    }
    pub async fn download(
        &self,
        client: &reqwest::Client,
        key: &str,
        destination: &Path,
        limit: i64,
    ) -> Result<i64> {
        if matches!(self, Self::B2 { .. }) {
            if let Some(error) = cooldown_error() {
                return Err(error);
            }
        }
        for attempt in 0..3 {
            match self.download_once(client, key, destination, limit).await {
                Err(error) if error.0 == StatusCode::BAD_GATEWAY && attempt < 2 => {
                    tracing::warn!(attempt=attempt+1,reason=%error.1,"Retrying staged-object read");
                    tokio::time::sleep(std::time::Duration::from_millis(250 << attempt)).await;
                }
                result => return result,
            }
        }
        unreachable!("final read attempt returns its result")
    }
    async fn download_once(
        &self,
        client: &reqwest::Client,
        key: &str,
        destination: &Path,
        limit: i64,
    ) -> Result<i64> {
        if matches!(self, Self::Local { .. }) {
            let size = tokio::fs::metadata(self.path(key)?).await?.len();
            if size > limit as u64 {
                return Err(Error::bad("Uploaded file exceeds allowed size"));
            }
            tokio::fs::copy(self.path(key)?, destination).await?;
            return Ok(size as i64);
        }
        let mut response = client
            .get(self.signed("GET", key, None)?)
            .send()
            .await
            .map_err(|_| storage_error())?;
        response = check_response(response, "download").await?;
        if response.content_length().is_some_and(|n| n > limit as u64) {
            return Err(Error::bad("Uploaded file exceeds allowed size"));
        }
        let mut out = tokio::fs::File::create(destination).await?;
        let mut size = 0_i64;
        while let Some(chunk) = response.chunk().await.map_err(|_| storage_error())? {
            size += chunk.len() as i64;
            if size > limit {
                return Err(Error::bad("Uploaded file exceeds allowed size"));
            }
            out.write_all(&chunk).await?
        }
        out.flush().await?;
        Ok(size)
    }
    pub async fn put_file(
        &self,
        client: &reqwest::Client,
        key: &str,
        source: &Path,
        size: i64,
    ) -> Result<()> {
        if matches!(self, Self::Local { .. }) {
            let path = self.path(key)?;
            tokio::fs::create_dir_all(path.parent().unwrap()).await?;
            tokio::fs::copy(source, path).await?;
            return Ok(());
        }
        let file = tokio::fs::File::open(source).await?;
        let result = client
            .put(self.signed("PUT", key, Some(size))?)
            .header("content-length", size)
            .body(reqwest::Body::wrap_stream(ReaderStream::new(file)))
            .send()
            .await
            .map_err(|_| storage_error())?;
        check_response(result, "upload").await?;
        Ok(())
    }
    pub async fn delete(&self, client: &reqwest::Client, key: &str) -> Result<()> {
        if matches!(self, Self::Local { .. }) {
            match tokio::fs::remove_file(self.path(key)?).await {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
            return Ok(());
        }
        for _ in 0..20 {
            let response = client
                .get(self.signed_query(
                    "GET",
                    "",
                    None,
                    &[
                        ("versions", String::new()),
                        ("prefix", key.into()),
                        ("max-keys", "1000".into()),
                    ],
                )?)
                .send()
                .await
                .map_err(|_| storage_error())?;
            let response = check_response(response, "list versions").await?;
            let xml = response.text().await.map_err(|_| storage_error())?;
            let versions: Versions = quick_xml::de::from_str(&xml).map_err(|_| storage_error())?;
            let exact: Vec<_> = versions
                .versions
                .into_iter()
                .chain(versions.markers)
                .filter(|v| v.key == key)
                .collect();
            if exact.is_empty() {
                return Ok(());
            }
            for version in exact {
                let response = client
                    .delete(self.signed_query("DELETE", key, None, &[("versionId", version.id)])?)
                    .send()
                    .await
                    .map_err(|_| storage_error())?;
                if response.status() != 404 {
                    check_response(response, "delete version").await?;
                }
            }
        }
        Err(storage_error())
    }
}
