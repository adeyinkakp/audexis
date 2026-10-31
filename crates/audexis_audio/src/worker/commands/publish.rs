use super::*;

pub(in crate::worker) fn publish_state<P>(ctx: &WorkerContext<P>, state: &WorkerState)
where
    P: Producer<Item = f32> + Observer + Send,
{
    let path = if state.format.is_some() {
        ctx.queue.lock().ok().and_then(|queue| queue.current_path())
    } else {
        None
    };
    let modes = ctx.queue.lock().unwrap().playback_modes();
    ctx.events.state.send_modify(|snapshot| {
        snapshot.shuffled = modes.shuffled;
        snapshot.repeat_mode = modes.repeat_mode;
        snapshot.current_position = ctx.position_ms.load(Ordering::Acquire);
        snapshot.duration = ctx.audio_duration.load(Ordering::Acquire);
        snapshot.id = ctx.database_id.load(Ordering::Acquire);
        snapshot.paused = ctx.paused.load(Ordering::Acquire);
        snapshot.path = path;
    });
}

pub(in crate::worker) fn publish_track<P>(
    ctx: &mut WorkerContext<P>,
    id: i64,
    duration: u64,
    playlist_id: Option<i64>,
    position: u64,
) where
    P: Producer<Item = f32> + Observer + Send,
{
    let path = ctx.queue.lock().ok().and_then(|queue| queue.current_path());
    ctx.events.state.send_modify(|snapshot| {
        snapshot.path = path.clone();
        snapshot.id = id;
        snapshot.duration = duration;
        snapshot.current_position = position;
        snapshot.error = None;
    });
    let title = path
        .as_deref()
        .and_then(|path| std::path::Path::new(path).file_stem())
        .map(|name| name.to_string_lossy().into_owned());
    update_controls_metadata(
        ctx,
        PartialMetadata {
            title,
            artist: None,
            album: None,
            cover_url: None,
            duration: Some(duration),
        },
    );
    ctx.database_id.store(id, Ordering::Release);
    ctx.audio_duration.store(duration, Ordering::Release);
    ctx.position_ms.store(position, Ordering::Release);
    if let Ok(mut counter) = ctx.play_counter.lock() {
        counter.start(id, duration, playlist_id);
    }
    ctx.events.emit(AudioEvent::TrackStarted { id, duration });
}

pub(in crate::worker) fn update_controls_metadata<P>(
    ctx: &mut WorkerContext<P>,
    metadata: PartialMetadata,
) where
    P: Producer<Item = f32> + Observer + Send,
{
    #[cfg(target_os = "macos")]
    let cover_url = None;

    #[cfg(not(target_os = "macos"))]
    let cover_url = metadata
        .cover_url
        .as_deref()
        .filter(|url| !url.trim().is_empty());

    match ctx.controls.lock() {
        Ok(mut controls) => {
            if let Err(error) = controls.set_metadata(MediaMetadata {
                title: metadata.title.as_deref(),
                album: metadata.album.as_deref(),
                artist: metadata.artist.as_deref(),
                cover_url,
                duration: metadata.duration.map(Duration::from_millis),
            }) {
                tracing::error!("Could not update media-control metadata: {error}");
            }
        }
        Err(_) => {
            tracing::error!("Media controls are unavailable");
        }
    }
}
