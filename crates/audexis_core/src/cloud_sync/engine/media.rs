use super::*;

impl CloudSyncContext {
    pub async fn local_audio(&self, track: &Track) -> Result<Option<PathBuf>> {
        if track.deleted {
            return Err(SyncError::local(
                "This track was removed from your cloud library",
            ));
        }
        let local:Option<String>=sqlx::query_scalar("SELECT f.path FROM file_cloud_links l JOIN files f ON f.id=l.file_id WHERE l.connection_id=? AND l.cloud_track_id=? AND f.cloud_connection_id IS NULL AND f.missing_since IS NULL ORDER BY f.id LIMIT 1").bind(&self.connection_id).bind(&track.id).fetch_optional(&self.db.pool).await?;
        if let Some(path) = local {
            if tokio::fs::metadata(&path).await.is_ok() {
                return Ok(Some(path.into()));
            }
        }
        let path = self.cache_path(track);
        if tokio::fs::metadata(&path)
            .await
            .is_ok_and(|m| m.len() == track.size as u64)
        {
            let check_path = path.clone();
            let expected = track.sha256.clone();
            if tokio::task::spawn_blocking(move || {
                use std::io::Read;
                let mut file = std::fs::File::open(check_path).ok()?;
                let mut hasher = Sha256::new();
                let mut buffer = [0u8; 65536];
                loop {
                    let n = file.read(&mut buffer).ok()?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buffer[..n]);
                }
                Some(format!("{:x}", hasher.finalize()) == expected)
            })
            .await
            .ok()
            .flatten()
                == Some(true)
            {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }
    pub async fn stream_url(&self, track: &Track) -> Result<String> {
        let media = self.stream_media(track, "original").await?;
        if media.url.is_empty() {
            return Err(SyncError::local(
                "Connect to your cloud server to download this track",
            ));
        }
        Ok(media.url)
    }
    pub async fn stream_media(&self, track: &Track, quality: &str) -> Result<MediaUrl> {
        if track.deleted {
            return Err(SyncError::local(
                "This track was removed from your cloud library",
            ));
        }
        if !matches!(quality, "original" | "aac_128" | "aac_256") {
            return Err(SyncError::local("Unknown streaming quality"));
        }
        let descriptor = self
            .cache_dir
            .join(&self.connection_id)
            .join("streaming_descriptors")
            .join(format!(
                "{:x}.json",
                Sha256::digest(format!("{}:{quality}", track.id).as_bytes())
            ));
        let remote = self.fresh_stream_media(track, quality).await;
        let media = match remote {
            Ok(media) => media,
            Err(error)
                if error.status.is_none()
                    || matches!(error.status, Some(429 | 502 | 503 | 504)) =>
            {
                match tokio::fs::read(&descriptor)
                    .await
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<MediaUrl>(&bytes).ok())
                {
                    Some(mut media) => {
                        media.url.clear();
                        media
                    }
                    None => return Err(error),
                }
            }
            Err(error) => return Err(error),
        };
        if media.quality == "original" && (media.sha256 != track.sha256 || media.size != track.size)
        {
            return Err(SyncError::local(
                "Cloud audio version changed. Sync before downloading.",
            ));
        }
        if media.size <= 0
            || media.sha256.len() != 64
            || !media.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(SyncError::local("Invalid cloud media descriptor"));
        }
        if !media.url.is_empty() {
            let mut saved = media.clone();
            saved.url.clear();
            if tokio::fs::create_dir_all(descriptor.parent().unwrap())
                .await
                .is_ok()
            {
                let temporary = descriptor.with_extension(format!("{}.part", uuid::Uuid::new_v4()));
                if let Ok(bytes) = serde_json::to_vec(&saved) {
                    if tokio::fs::write(&temporary, bytes).await.is_ok() {
                        let _ = tokio::fs::rename(&temporary, &descriptor).await;
                    }
                }
                let _ = tokio::fs::remove_file(temporary).await;
            }
        }
        Ok(media)
    }
    pub async fn fresh_stream_media(&self, track: &Track, quality: &str) -> Result<MediaUrl> {
        let client = self.client()?;
        client.verify(&self.server_user_id).await?;
        client.stream_media(&track.id, quality).await
    }
    pub async fn download(&self, track: &Track) -> Result<PathBuf> {
        if let Some(path) = self.local_audio(track).await? {
            if path == self.cache_path(track) {
                self.record_download(track).await?;
                self.files.publish_changed(Vec::new());
            }
            return Ok(path);
        }
        let path = self.cache_path(track);
        let url = self.stream_url(track).await?;
        self.client()?
            .download(&url, &path, Some(&track.sha256), track.size)
            .await
            .map_err(|error| {
                tracing::warn!(track_id=%track.id, status=?error.status, reason=%error, "Cloud playback download failed");
                error
            })?;
        self.record_download(track).await?;
        self.files.publish_changed(Vec::new());
        Ok(path)
    }
    pub async fn remove_download(&self, track: &Track) -> Result<()> {
        match tokio::fs::remove_file(self.cache_path(track)).await {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        sqlx::query("DELETE FROM cloud_downloads WHERE connection_id=? AND track_id=?")
            .bind(&self.connection_id)
            .bind(&track.id)
            .execute(&self.db.pool)
            .await?;
        self.files.publish_changed(Vec::new());
        Ok(())
    }
}
