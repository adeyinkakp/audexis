use super::*;

pub(in crate::worker) fn handle_track_completion<P>(
    ctx: &mut WorkerContext<P>,
    state: &mut WorkerState,
) where
    P: Producer<Item = f32> + Observer + Send,
{
    let mut q = ctx.queue.lock().unwrap();
    if q.tracks.get(q.index as usize).is_none() {
        if let Ok(mut counter) = ctx.play_counter.lock() {
            counter.stop();
        }
        q.clear();
        let modes = q.playback_modes();
        drop(q);
        ctx.paused.store(true, Ordering::Release);
        ctx.flush_output.store(true, Ordering::Release);
        ctx.position_ms.store(0, Ordering::Release);
        ctx.audio_duration.store(0, Ordering::Release);
        ctx.database_id.store(0, Ordering::Relaxed);
        state.clear_loaded_track();
        clear_now_playing(&ctx.events);
        ctx.events.emit(AudioEvent::QueueChanged(QueueInfo {
            queue_ids: Vec::new(),
            paths: Vec::new(),
            file_ids: Vec::new(),
            occurrences: Vec::new(),
            current_index: 0,
            playlist_id: None,
        }));
        ctx.events.emit(AudioEvent::ModesChanged(modes));
        ctx.events.emit(AudioEvent::QueueFinished);
        if let Ok(mut controls) = ctx.controls.lock() {
            let _ = controls.set_playback(MediaPlayback::Stopped);
            let _ = controls.set_metadata(MediaMetadata {
                title: Some(""),
                artist: Some(""),
                album: Some(""),
                ..MediaMetadata::default()
            });
        }
        return;
    }

    let queue_info = QueueInfo {
        queue_ids: q.queue_ids(),
        paths: q.paths(),
        file_ids: q.file_ids(),
        occurrences: q.playlist_ordinals(),
        current_index: q.index,
        playlist_id: q.playlist_id,
    };
    drop(q);
    ctx.events.emit(AudioEvent::QueueChanged(queue_info));
    let _ = ctx.cmd_tx.send(PlayerCmd::Play { resuming: false });
    ctx.events.emit(AudioEvent::TrackFinished);
    state.is_done = false;
}

pub(in crate::worker) fn fail_playback<P>(
    ctx: &mut WorkerContext<P>,
    state: &mut WorkerState,
    error: &str,
) where
    P: Producer<Item = f32> + Observer + Send,
{
    let file = state
        .transitions
        .back()
        .map(|transition| transition.queue_id.clone());
    let mut queue = ctx.queue.lock().unwrap();
    let path = file
        .and_then(|id| {
            queue.tracks.iter().find_map(|track| {
                let track = track.lock().ok()?;
                (track.queue_id == id).then(|| track.path.clone())
            })
        })
        .or_else(|| queue.current_path())
        .unwrap_or_default();
    queue.clear();
    drop(queue);

    handle_track_completion(ctx, state);
    let message = format!("Playback stopped: {path}: {error}");
    tracing::error!(%message);
    ctx.events
        .state
        .send_modify(|state| state.error = Some(message.clone()));
    ctx.events.emit(AudioEvent::Error(message));
}

pub(in crate::worker) fn clear_now_playing(events: &crate::events::EventBus) {
    events.state.send_replace(crate::PlaybackState::default());
}

pub(in crate::worker) fn handle_play<P>(
    ctx: &mut WorkerContext<P>,
    state: &mut WorkerState,
    resuming: bool,
) where
    P: Producer<Item = f32> + Observer + Send,
{
    if resuming && state.format.is_some() && state.decoder.is_some() {
        ctx.paused.store(false, Ordering::Release);
        return;
    }
    ctx.paused.store(true, Ordering::Release);
    ctx.flush_output.store(true, Ordering::Release);
    ctx.position_ms.store(0, Ordering::Release);
    ctx.audio_duration.store(0, Ordering::Release);
    ctx.database_id.store(0, Ordering::Release);
    if let Ok(mut counter) = ctx.play_counter.lock() {
        counter.stop();
    }
    state.clear_loaded_track();
    let q = ctx.queue.lock().unwrap();
    let Some(track) = q.tracks.get(q.index as usize) else {
        drop(q);
        handle_track_completion(ctx, state);
        return;
    };
    let track = Arc::clone(track);
    let track = track.lock().unwrap();
    let path = track.path.clone();
    let id = track.database_id;
    let playlist_id = q.playlist_id.filter(|_| track.playlist_ord.is_some());
    drop(track);
    drop(q);
    match super::super::source::open(&path, state.target_sample_rate, state.output_channels) {
        Ok((loaded, duration)) => {
            *state = loaded;
            publish_track(ctx, id, duration, playlist_id, 0);
            ctx.paused.store(false, Ordering::Release);
        }
        Err(error) => fail_playback(ctx, state, &error),
    }
}
