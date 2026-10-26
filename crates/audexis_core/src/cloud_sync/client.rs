mod error;
pub use error::*;

mod response;
use response::*;

use super::protocol::*;
use reqwest::{Client, Method};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

#[derive(Clone)]
pub struct CloudClient {
    base: String,
    key: String,
    client: Client,
}
impl CloudClient {
    pub fn new(base: &str, key: &str) -> Result<Self> {
        let url =
            reqwest::Url::parse(base).map_err(|_| SyncError::local("Invalid cloud server URL"))?;
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(SyncError::local("Invalid cloud server URL"));
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| SyncError::local("Could not initialize sync client"))?;
        Ok(Self {
            base: base.trim_end_matches('/').into(),
            key: key.into(),
            client,
        })
    }
    async fn response(
        &self,
        method: Method,
        path: &str,
        body: Option<&impl Serialize>,
    ) -> Result<reqwest::Response> {
        let mut request = self
            .client
            .request(method, format!("{}/api{path}", self.base))
            .timeout(Duration::from_secs(30))
            .bearer_auth(&self.key);
        if let Some(body) = body {
            request = request.json(body)
        }
        let response = request
            .send()
            .await
            .map_err(|_| SyncError::local("Could not reach your cloud server"))?;
        check(response).await.map_err(|mut error| {
            error.message = format!(
                "{}: {}",
                path.split('?').next().unwrap_or(path),
                error.message
            );
            error
        })
    }
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        Self::decode(self.response(Method::GET, path, None::<&()>).await?).await
    }
    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        Self::decode(self.response(Method::POST, path, Some(body)).await?).await
    }
    async fn decode<T: DeserializeOwned>(mut response: reqwest::Response) -> Result<T> {
        const MAX_BYTES: usize = 8 * 1024 * 1024;
        if response
            .content_length()
            .is_some_and(|size| size > MAX_BYTES as u64)
        {
            return Err(SyncError::local("Cloud response exceeds the size limit"));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| SyncError::local("Could not read cloud response"))?
        {
            if chunk.len() > MAX_BYTES - bytes.len() {
                return Err(SyncError::local("Cloud response exceeds the size limit"));
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| SyncError::local("Invalid response from cloud server"))
    }
    pub async fn cancel_upload(&self, id: &str) -> Result<()> {
        self.response(Method::DELETE, &format!("/uploads/{id}"), None::<&()>)
            .await?;
        Ok(())
    }
    pub async fn upload(&self, url: &str, path: &Path, size: i64) -> Result<()> {
        self.upload_with_progress(url, path, size, |_| {}).await
    }
    pub async fn upload_with_progress(
        &self,
        url: &str,
        path: &Path,
        size: i64,
        report: impl Fn(u64) + Send + Sync + 'static,
    ) -> Result<()> {
        media_url(url)?;
        let file = tokio::fs::File::open(path).await?;
        let response = self
            .client
            .put(url)
            .header("content-length", size)
            .body(reqwest::Body::wrap_stream(ReaderStream::new(
                UploadReader {
                    file,
                    uploaded: 0,
                    report: Box::new(report),
                    last_report: std::time::Instant::now(),
                },
            )))
            .send()
            .await
            .map_err(|_| SyncError::local("Audio upload interrupted"))?;
        check(response).await?;
        Ok(())
    }
    pub async fn download(
        &self,
        url: &str,
        destination: &Path,
        hash: Option<&str>,
        limit: i64,
    ) -> Result<()> {
        media_url(url)?;
        let mut response = self
            .client
            .get(url)
            .timeout(Duration::from_secs(if hash.is_some() { 300 } else { 15 }))
            .send()
            .await
            .map_err(|_| SyncError::local("Download interrupted"))?;
        response = check_media(response).await?;
        if response.content_length().is_some_and(|n| n > limit as u64) {
            return Err(SyncError::local("Cloud media exceeds its declared size"));
        }
        tokio::fs::create_dir_all(destination.parent().unwrap()).await?;
        let temp = destination.with_extension(format!("{}.part", uuid::Uuid::new_v4()));
        let result = async {
            let mut file = tokio::fs::File::create(&temp).await?;
            let mut digest = Sha256::new();
            let mut size = 0_i64;
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| SyncError::local("Download interrupted"))?
            {
                size += chunk.len() as i64;
                if size > limit {
                    return Err(SyncError::local("Cloud media exceeds its declared size"));
                }
                digest.update(&chunk);
                file.write_all(&chunk).await?;
            }
            if let Some(hash) = hash {
                if format!("{:x}", digest.finalize()) != hash || size != limit {
                    return Err(SyncError::local("Downloaded audio failed verification"));
                }
            }
            file.flush().await?;
            drop(file);
            tokio::fs::rename(&temp, destination).await?;
            Ok(())
        }
        .await;
        if result.is_err() {
            let _ = tokio::fs::remove_file(&temp).await;
        }
        result
    }
    pub async fn verify(&self, expected_user: &str) -> Result<()> {
        #[derive(serde::Deserialize)]
        struct Identity {
            authenticated: bool,
            user_id: String,
        }
        let identity: Identity = self.get("/session").await?;
        if !identity.authenticated || identity.user_id != expected_user {
            return Err(SyncError::local(
                "The server account changed. Sign in again before syncing.",
            ));
        }
        Ok(())
    }
    pub async fn media(&self, id: &str, kind: &str) -> Result<MediaUrl> {
        if !safe_id(id) {
            return Err(SyncError::local("Invalid cloud track ID"));
        }
        self.get(&format!("/tracks/{id}/{kind}")).await
    }
    pub async fn stream_media(&self, id: &str, quality: &str) -> Result<MediaUrl> {
        if !safe_id(id) || !matches!(quality, "original" | "aac_128" | "aac_256") {
            return Err(SyncError::local("Invalid streaming request"));
        }
        self.get(&format!("/tracks/{id}/audio?quality={quality}"))
            .await
    }
}
pub fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
}
struct UploadReader {
    file: tokio::fs::File,
    uploaded: u64,
    report: Box<dyn Fn(u64) + Send + Sync>,
    last_report: std::time::Instant,
}
impl tokio::io::AsyncRead for UploadReader {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        match std::pin::Pin::new(&mut this.file).poll_read(cx, buf) {
            std::task::Poll::Ready(Ok(())) => {
                let bytes = (buf.filled().len() - before) as u64;
                this.uploaded = this.uploaded.saturating_add(bytes);
                if bytes == 0 || this.last_report.elapsed() >= Duration::from_millis(150) {
                    (this.report)(this.uploaded);
                    this.last_report = std::time::Instant::now();
                }
                std::task::Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}
