use ringbuf::traits::{Observer, Producer};
use souvlaki::{MediaMetadata, MediaPlayback};
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;
use symphonia::core::formats::TrackType;
use symphonia::core::formats::{probe::Hint, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::units::{Time, Timestamp};
use tauri::Emitter;

use super::{WorkerContext, WorkerState};
use crate::audio_player::duration::resolve_duration_ms;
use crate::audio_player::types::{
    AudioPlayerError, PartialMetadata, PartialPlaybackInfo, PlayerCmd,
};
use crate::commands::playback::shared::QueueInfo;

pub fn handle_track_completion<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState)
where
    P: Producer<Item = f32> + Observer + Send,
{
    let mut q = ctx.queue.lock().unwrap();
    let current_t = q.tracks.get(q.index as usize);
    if current_t.is_none() {
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
        let _ = ctx
            .controls
            .lock()
            .unwrap()
            .set_playback(MediaPlayback::Stopped);
        let _ = ctx
            .controls
            .lock()
            .unwrap()
            .set_metadata(MediaMetadata::default());
        let _ = ctx.app_handle.emit(
            "queue-changed",
            QueueInfo {
                paths: Vec::new(),
                file_ids: Vec::new(),
                occurrences: Vec::new(),
                current_index: 0,
                playlist_id: None,
            },
        );
        let _ = ctx.app_handle.emit("playback-modes-changed", modes);
        let _ = ctx.app_handle.emit("playback-queue-done", ());
        return;
    }

    drop(q);
    let _ = ctx.cmd_tx.send(PlayerCmd::Play { resuming: false });
    let _ = ctx.app_handle.emit("playback-media-done", ());
    state.is_done = false;
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
            state.clear_loaded_track();
        }
        PlayerCmd::Seek { seconds } => handle_seek(ctx, state, seconds),
        PlayerCmd::UpdateDeviceConfig { target_sample_rate } => {
            state.target_sample_rate = target_sample_rate;
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

    if !resuming {
        if let Ok(mut counter) = ctx.play_counter.lock() {
            counter.stop();
        }
        ctx.paused.store(true, Ordering::Release);
        ctx.flush_output.store(true, Ordering::Release);
        ctx.position_ms.store(0, Ordering::Release);
        ctx.audio_duration.store(0, Ordering::Release);
        ctx.database_id.store(0, Ordering::Relaxed);
    }
    let q = ctx.queue.lock().unwrap();
    let current_t = q.tracks.get(q.index as usize);
    if current_t.is_none() {
        let _ = ctx.app_handle.emit("playback-queue-done", ());
        return;
    }

    let playlist_id = q.playlist_id.filter(|_| {
        current_t
            .unwrap()
            .lock()
            .ok()
            .is_some_and(|track| track.playlist_ord.is_some())
    });
    let current_track = Arc::clone(current_t.unwrap());
    let media_database_id = current_track.lock().unwrap().database_id;
    ctx.database_id.store(media_database_id, Ordering::Relaxed);
    drop(q);

    state.clear_playback_buffers();

    let file_path = current_track.lock().unwrap().path.clone();
    let src = match std::fs::File::open(&file_path) {
        Ok(src) => src,
        Err(error) => {
            tauri_plugin_log::log::error!(
                "{}",
                AudioPlayerError::FileOpen {
                    path: file_path.clone(),
                    source: error,
                }
            );
            return;
        }
    };

    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = std::path::Path::new(&file_path)
        .extension()
        .and_then(|e| e.to_str())
    {
        hint.with_extension(ext);
    }

    let mut probed = match symphonia::default::get_probe().probe(
        &hint,
        mss,
        Default::default(),
        Default::default(),
    ) {
        Ok(probed) => probed,
        Err(error) => {
            tauri_plugin_log::log::error!(
                "{}",
                AudioPlayerError::Probe {
                    path: file_path.clone(),
                    message: error.to_string(),
                }
            );
            return;
        }
    };

    let (track_id, sample_rate, channel_count, decoder) = {
        let track = match probed.default_track(TrackType::Audio) {
            Some(track) => track,
            None => {
                tauri_plugin_log::log::error!(
                    "{}",
                    AudioPlayerError::MissingTrack { path: file_path }
                );
                return;
            }
        };

        let track_id = track.id;
        let Some(audio_codec_params) = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
        else {
            tauri_plugin_log::log::error!(
                "{}",
                AudioPlayerError::MissingTrack {
                    path: file_path.clone(),
                }
            );
            return;
        };

        let Some(sample_rate) = audio_codec_params.sample_rate else {
            tauri_plugin_log::log::error!(
                "{}",
                AudioPlayerError::MissingSampleRate {
                    path: file_path.clone(),
                }
            );
            return;
        };

        let Some(channels) = audio_codec_params.channels.as_ref() else {
            tauri_plugin_log::log::error!(
                "{}",
                AudioPlayerError::MissingChannels {
                    path: file_path.clone(),
                }
            );
            return;
        };

        let decoder = match symphonia::default::get_codecs()
            .make_audio_decoder(audio_codec_params, &Default::default())
        {
            Ok(decoder) => decoder,
            Err(error) => {
                tauri_plugin_log::log::error!(
                    "{}",
                    AudioPlayerError::Decoder {
                        path: file_path.clone(),
                        message: error.to_string(),
                    }
                );
                return;
            }
        };

        (track_id, sample_rate, channels.count(), decoder)
    };

    state.track_id = track_id;

    let duration = resolve_duration_ms(&mut probed, state.track_id, None);
    ctx.audio_duration.store(duration.ms, Ordering::Release);
    if let Ok(mut counter) = ctx.play_counter.lock() {
        counter.start(media_database_id, duration.ms, playlist_id);
    }

    if let Err(err) = ctx.app_handle.emit(
        "playback-media-start",
        PartialPlaybackInfo {
            id: media_database_id,
            duration: duration.ms,
        },
    ) {
        println!(":{:?}", err);
    }

    state.source_sample_rate = sample_rate;
    state.channels_uz = channel_count;
    state.decoder = Some(decoder);
    state.format = Some(probed);
    state.resampler = None;
    ctx.paused.store(false, Ordering::Release);
}

fn handle_seek<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState, seconds: u64)
where
    P: Producer<Item = f32> + Observer + Send,
{
    let was_paused = ctx.paused.swap(true, Ordering::AcqRel);
    if let Some(current_format) = state.format.as_mut() {
        let target_timestamp = current_format
            .tracks()
            .iter()
            .find(|track| track.id == state.track_id)
            .and_then(|track| {
                let time_base = track.time_base?;
                let requested = time_base
                    .calc_timestamp(Time::try_new(seconds as i64, 0).unwrap_or(Time::MAX))?;
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
                    time: Time::try_new(seconds as i64, 0).unwrap_or(Time::MAX),
                    track_id: None,
                },
            )
        };

        if let Err(error) = seek_result {
            tauri_plugin_log::log::error!("Could not seek playback: {error}");
        } else if let Some(current_decoder) = state.decoder.as_mut() {
            if let Ok(mut counter) = ctx.play_counter.lock() {
                counter.seek(seconds.saturating_mul(1000));
            }
            current_decoder.reset();
            state.clear_playback_buffers();
            state.resampler = None;
            ctx.flush_output.store(true, Ordering::Release);
            ctx.position_ms
                .store(seconds.saturating_mul(1000), Ordering::Release);
        }
    }
    ctx.paused.store(was_paused, Ordering::Release);
}

fn update_controls_metadata<P>(ctx: &mut WorkerContext<P>, metadata: PartialMetadata)
where
    P: Producer<Item = f32> + Observer + Send,
{
    let res = ctx.controls.lock().unwrap().set_metadata(MediaMetadata {
        title: metadata.title.as_deref(),
        album: metadata.album.as_deref(),
        artist: metadata.artist.as_deref(),
        cover_url: metadata.cover_url.as_deref(),
        duration: metadata.duration.map(Duration::from_millis),
    });
    if let Err(err) = res {
        tauri_plugin_log::log::error!("Error occurred: {}", err);
    }
}
