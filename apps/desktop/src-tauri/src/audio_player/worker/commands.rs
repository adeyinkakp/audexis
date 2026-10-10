use ringbuf::traits::{Observer, Producer};
use souvlaki::{MediaMetadata, MediaPlayback};
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;
use symphonia::core::formats::{SeekMode, SeekTo};
use symphonia::core::units::{Time, Timestamp};
use tauri::{Emitter, Manager};

use super::{WorkerContext, WorkerState};
use crate::audio_player::types::{PartialMetadata, PartialPlaybackInfo, PlayerCmd};
use crate::commands::playback::shared::QueueInfo;

pub fn handle_track_completion<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState)
where
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
        clear_now_playing(&ctx.app_handle);
        let _ = ctx.app_handle.emit(
            "queue-changed",
            QueueInfo {
                queue_ids: Vec::new(),
                paths: Vec::new(),
                file_ids: Vec::new(),
                occurrences: Vec::new(),
                current_index: 0,
                playlist_id: None,
            },
        );
        let _ = ctx.app_handle.emit("playback-modes-changed", modes);
        let _ = ctx.app_handle.emit("playback-queue-done", ());
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
    let _ = ctx.app_handle.emit("queue-changed", queue_info);
    let _ = ctx.cmd_tx.send(PlayerCmd::Play { resuming: false });
    let _ = ctx.app_handle.emit("playback-media-done", ());
    state.is_done = false;
}

pub(super) fn fail_playback<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState, error: &str)
where
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
    crate::utils::errors::report_backend_error(
        &ctx.app_handle,
        "AudioDecoderError",
        "Playback stopped because this track could not be decoded. Try another track.",
        format!("{path}: {error}"),
    );
}

fn clear_now_playing(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<crate::AppState>() {
        if let Ok(mut now_playing) = state.now_playing.lock() {
            *now_playing = None;
        }
    }
    let _ = app.emit(
        "now-playing-changed",
        Option::<crate::commands::playback::shared::NowPlayingInfo>::None,
    );
}

pub fn handle_command<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState, cmd: PlayerCmd)
where
    P: Producer<Item = f32> + Observer + Send,
{
    match cmd {
        PlayerCmd::Play { resuming } => handle_play(ctx, state, resuming),
        PlayerCmd::Pause => {
            ctx.paused.store(true, Ordering::Release);
            if let Ok(mut counter) = ctx.play_counter.lock() {
                counter.flush_time();
            }
        }
        PlayerCmd::Stop => {
            if let Ok(mut counter) = ctx.play_counter.lock() {
                counter.stop();
            }
            ctx.paused.store(true, Ordering::Release);
            ctx.flush_output.store(true, Ordering::Release);
            ctx.position_ms.store(0, Ordering::Release);
            ctx.audio_duration.store(0, Ordering::Release);
            ctx.database_id.store(0, Ordering::Relaxed);
            state.clear_loaded_track();
        }
        PlayerCmd::Seek { seconds } => handle_seek(ctx, state, seconds.saturating_mul(1000)),
        PlayerCmd::SeekMilliseconds { milliseconds } => handle_seek(ctx, state, milliseconds),
        PlayerCmd::UpdateDeviceConfig {
            target_sample_rate,
            output_channels,
        } => {
            let position = ctx.position_ms.load(Ordering::Acquire);
            handle_seek(ctx, state, position);
            state.target_sample_rate = target_sample_rate;
            state.output_channels = output_channels;
            state.resampler = None;
        }
        PlayerCmd::Preload => {}
        PlayerCmd::UpdateControlsMetadata { metadata } => update_controls_metadata(ctx, metadata),
    }
}

fn handle_play<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState, resuming: bool)
where
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
    match super::source::open(&path, state.target_sample_rate, state.output_channels) {
        Ok((loaded, duration)) => {
            *state = loaded;
            publish_track(ctx, id, duration, playlist_id, 0);
            ctx.paused.store(false, Ordering::Release);
        }
        Err(error) => fail_playback(ctx, state, &error),
    }
}

pub(super) fn publish_track<P>(
    ctx: &mut WorkerContext<P>,
    id: i64,
    duration: u64,
    playlist_id: Option<i64>,
    position: u64,
) where
    P: Producer<Item = f32> + Observer + Send,
{
    ctx.database_id.store(id, Ordering::Release);
    ctx.audio_duration.store(duration, Ordering::Release);
    ctx.position_ms.store(position, Ordering::Release);
    if let Ok(mut counter) = ctx.play_counter.lock() {
        counter.start(id, duration, playlist_id);
    }
    let _ = ctx
        .app_handle
        .emit("playback-media-start", PartialPlaybackInfo { id, duration });
}

pub(super) fn handle_seek<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState, milliseconds: u64)
where
    P: Producer<Item = f32> + Observer + Send,
{
    if let Err(error) = crate::audio_player::decoder_guard::guard(|| {
        seek_inner(ctx, state, milliseconds);
        Ok(())
    }) {
        fail_playback(ctx, state, &error);
    }
}

fn seek_inner<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState, milliseconds: u64)
where
    P: Producer<Item = f32> + Observer + Send,
{
    state.restore_audible_track();
    let was_paused = ctx.paused.swap(true, Ordering::AcqRel);
    if let Some(current_format) = state.format.as_mut() {
        let target_timestamp = current_format
            .tracks()
            .iter()
            .find(|track| track.id == state.track_id)
            .and_then(|track| {
                let time_base = track.time_base?;
                let requested = time_base.calc_timestamp(
                    Time::try_new(
                        (milliseconds / 1000) as i64,
                        ((milliseconds % 1000) * 1_000_000) as u32,
                    )
                    .unwrap_or(Time::MAX),
                )?;
                let target = track.duration.map_or(requested, |duration| {
                    println!("duration : {:?}", duration);
                    let maximum = Timestamp::new(duration.get() as i64);
                    if requested.get() > maximum.get() {
                        maximum
                    } else {
                        requested
                    }
                });
                Some(target)
            });

        let seek_result = if let Some(target) = target_timestamp {
            current_format.seek(
                SeekMode::Accurate,
                SeekTo::Timestamp {
                    ts: target,
                    track_id: state.track_id,
                },
            )
        } else {
            current_format.seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::try_new(
                        (milliseconds / 1000) as i64,
                        ((milliseconds % 1000) * 1_000_000) as u32,
                    )
                    .unwrap_or(Time::MAX),
                    track_id: None,
                },
            )
        };

        if let Err(error) = seek_result {
            tauri_plugin_log::log::error!("Could not seek playback: {error}");
        } else if let Some(current_decoder) = state.decoder.as_mut() {
            if let Ok(mut counter) = ctx.play_counter.lock() {
                counter.seek(milliseconds);
            }
            current_decoder.reset();
            state.clear_playback_buffers();
            state.resampler = None;
            ctx.flush_output.store(true, Ordering::Release);
            ctx.position_ms.store(milliseconds, Ordering::Release);
        }
    }
    ctx.paused.store(was_paused, Ordering::Release);
}

fn update_controls_metadata<P>(ctx: &mut WorkerContext<P>, metadata: PartialMetadata)
where
    P: Producer<Item = f32> + Observer + Send,
{
    // Panics sometimes and unable to handle err due to crate
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
                tauri_plugin_log::log::error!("Could not update media-control metadata: {error}");
            }
        }
        Err(_) => {
            tauri_plugin_log::log::error!("Media controls are unavailable");
        }
    }
}
