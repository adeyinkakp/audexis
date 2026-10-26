use super::*;

impl AppServices {
    pub fn install_library_playback(self: &Arc<Self>) {
        let services = Arc::downgrade(self);
        let runtime = tokio::runtime::Handle::current();
        audexis_audio::streaming::set_library_resolver(move |path| {
            let services = services.upgrade().ok_or("Library unavailable")?;
            let context = services.cloud_context()?;
            let (quality, cache_mb) = services
                .user_config
                .lock()
                .map(|c| (c.cloud_audio_quality.api_value(), c.playback_cache_mb))
                .unwrap_or(("original", 2048));
            let track = runtime
                .block_on(context.library_track(path))
                .map_err(|e| e.to_string())?;
            if let Some(local) = runtime
                .block_on(context.local_audio(&track))
                .map_err(|e| e.to_string())?
            {
                return Ok(audexis_audio::streaming::LibrarySource::Local(local));
            }
            let media = runtime
                .block_on(context.stream_media(&track, quality))
                .map_err(|e| e.to_string())?;
            let extension = if media.quality != "original" {
                "m4a".into()
            } else {
                std::path::Path::new(&track.file_name)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("audio")
                    .to_owned()
            };
            let options = audexis_audio::streaming::StreamOptions {
                cache_dir: Some(
                    context
                        .cache_dir
                        .join(&context.connection_id)
                        .join("streaming"),
                ),
                cache_key: format!("{}:{}:{}", track.id, media.sha256, media.quality),
                cache_bytes: cache_mb.min(102400).saturating_mul(1024 * 1024),
                prefetch: true,
            };
            let expected = (media.sha256.clone(), media.size);
            let selected = media.quality.clone();
            let first = Mutex::new((!media.url.is_empty()).then_some(media.url));
            let resolver_runtime = runtime.clone();
            let weak = Arc::downgrade(&services);
            let connection = context.connection_id.clone();
            let source = audexis_audio::streaming::StreamRegistration::with_options(
                media.size.max(0) as u64,
                track.duration_ms.unwrap_or(0).max(0) as u64,
                extension,
                options,
                move || {
                    let services = weak.upgrade().ok_or("Library unavailable")?;
                    let context = services.cloud_context()?;
                    if context.connection_id != connection {
                        return Err("Cloud account changed".into());
                    }
                    if let Some(url) = first.lock().map_err(|_| "Stream unavailable")?.take() {
                        return Ok(url);
                    }
                    let media = resolver_runtime
                        .block_on(context.fresh_stream_media(&track, &selected))
                        .map_err(|e| e.to_string())?;
                    if media.url.is_empty() {
                        return Err(
                            "This part of the song isn't cached. Reconnect to continue.".into()
                        );
                    }
                    if (media.sha256, media.size) != expected {
                        return Err("Cloud audio changed. Play the song again.".into());
                    }
                    Ok(media.url)
                },
            );
            Ok(audexis_audio::streaming::LibrarySource::Stream(source))
        });
    }
}
