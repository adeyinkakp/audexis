use ringbuf::traits::{Observer, Producer};
use souvlaki::{MediaMetadata, MediaPlayback};
use std::sync::{Arc, atomic::Ordering};
use std::time::Duration;
use symphonia::core::formats::{SeekMode, SeekTo};
use symphonia::core::units::{Time, Timestamp};

use super::{WorkerContext, WorkerState};
use crate::events::{AudioEvent, QueueInfo};
use crate::types::{PartialMetadata, PlayerCmd};

pub fn handle_command<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState, cmd: PlayerCmd)
where
    P: Producer<Item = f32> + Observer + Send,
{
    match cmd {
        PlayerCmd::RemoveMissingFiles { file_ids } => {
            handle_command(
                ctx,
                state,
                PlayerCmd::ReconcileLibraryFiles {
                    path_updates: Default::default(),
                    file_ids,
                },
            );
        }
        PlayerCmd::ReconcileLibraryFiles {
            path_updates,
            file_ids,
        } => {
            let missing = file_ids.into_iter().collect();
            let mut queue = ctx.queue.lock().unwrap();
            let outcome = queue.reconcile_library_files(&path_updates, &missing);
            drop(queue);
            let was_paused = ctx.paused.load(Ordering::Acquire);
            if outcome.current_removed {
                if outcome.current_path.is_none() {
                    handle_track_completion(ctx, state);
                } else if state.format.is_some() {
                    handle_play(ctx, state, false);
                    if was_paused {
                        ctx.paused.store(true, Ordering::Release);
                    }
                } else {
                    handle_command(ctx, state, PlayerCmd::Stop);
                }
            } else if outcome.current_path_changed && state.format.is_some() {
                let position = ctx.position_ms.load(Ordering::Acquire);
                handle_play(ctx, state, false);
                handle_seek(ctx, state, position);
                if was_paused {
                    ctx.paused.store(true, Ordering::Release);
                }
            } else if state.transitions.iter().any(|t| {
                missing.contains(&t.database_id) || path_updates.contains_key(&t.database_id)
            }) {
                handle_seek(ctx, state, ctx.position_ms.load(Ordering::Acquire));
            }
            let queue = ctx.queue.lock().unwrap();
            let info = QueueInfo {
                queue_ids: queue.queue_ids(),
                paths: queue.paths(),
                file_ids: queue.file_ids(),
                occurrences: queue.playlist_ordinals(),
                current_index: queue.index,
                playlist_id: queue.playlist_id,
            };
            drop(queue);
            ctx.events.emit(AudioEvent::QueueChanged(info));
        }
        PlayerCmd::PlayQueueEntry { queue_id } => {
            let mut queue = ctx.queue.lock().unwrap();
            let index = queue.queue_ids().iter().position(|id| id == &queue_id);
            if let Some(index) = index {
                queue.index = index as i32;
            }
            drop(queue);
            if index.is_some() {
                handle_play(ctx, state, false);
            }
        }
        PlayerCmd::RemoveQueueEntry { queue_id } => {
            let was_paused = ctx.paused.load(Ordering::Acquire);
            let removed = ctx
                .queue
                .lock()
                .unwrap()
                .remove_queue_entries(std::slice::from_ref(&queue_id));
            match removed {
                Ok(true) => {
                    if state.format.is_some() {
                        handle_play(ctx, state, false);
                    }
                    if was_paused {
                        ctx.paused.store(true, Ordering::Release);
                    }
                }
                Ok(false) => {
                    if state.transitions.iter().any(|t| t.queue_id == queue_id) {
                        handle_seek(ctx, state, ctx.position_ms.load(Ordering::Acquire));
                    }
                }
                Err(error) => {
                    ctx.events
                        .state
                        .send_modify(|s| s.error = Some(error.clone()));
                    ctx.events.emit(AudioEvent::Error(error));
                }
            }
        }
        PlayerCmd::ReorderQueueEntry {
            queue_id,
            target_id,
        } => {
            let result = ctx.queue.lock().unwrap().reorder(&queue_id, &target_id);
            if let Err(error) = result {
                ctx.events
                    .state
                    .send_modify(|s| s.error = Some(error.clone()));
                ctx.events.emit(AudioEvent::Error(error));
            } else if !state.transitions.is_empty() {
                handle_seek(ctx, state, ctx.position_ms.load(Ordering::Acquire));
            }
        }
        PlayerCmd::ClearQueue => {
            ctx.queue.lock().unwrap().clear();
            handle_track_completion(ctx, state);
        }
        PlayerCmd::ToggleShuffle => {
            let mut queue = ctx.queue.lock().unwrap();
            queue.toggle_shuffle();
            ctx.events
                .emit(AudioEvent::ModesChanged(queue.playback_modes()));
        }
        PlayerCmd::CycleRepeat => {
            let mut queue = ctx.queue.lock().unwrap();
            let next = match queue.repeat_mode {
                crate::RepeatMode::Off => crate::RepeatMode::Queue,
                crate::RepeatMode::Queue => crate::RepeatMode::Track,
                crate::RepeatMode::Track => crate::RepeatMode::Off,
            };
            queue.set_repeat_mode(next);
            ctx.events
                .emit(AudioEvent::ModesChanged(queue.playback_modes()));
        }
        PlayerCmd::Enqueue { tracks, next } => {
            if !tracks.is_empty() {
                let mut queue = ctx.queue.lock().unwrap();
                let empty = queue.tracks.is_empty();
                if next && !empty {
                    for track in tracks.into_iter().rev() {
                        queue.insert_next(track);
                    }
                } else {
                    for track in tracks {
                        queue.append(track);
                    }
                }
                drop(queue);
                if empty {
                    handle_play(ctx, state, false);
                } else if next && !state.transitions.is_empty() {
                    handle_seek(ctx, state, ctx.position_ms.load(Ordering::Acquire));
                }
            }
        }
        PlayerCmd::LoadAt { tracks, index } => {
            if index >= tracks.len() {
                let error = "Selected song is no longer in the playback queue".to_string();
                ctx.events
                    .state
                    .send_modify(|state| state.error = Some(error.clone()));
                ctx.events.emit(AudioEvent::Error(error));
            } else {
                ctx.queue.lock().unwrap().replace_with(tracks, index);
                ctx.events.state.send_modify(|state| state.error = None);
                handle_play(ctx, state, false);
            }
        }
        PlayerCmd::Load { tracks } => {
            let mut queue = ctx.queue.lock().unwrap();
            queue.clear();
            for track in tracks {
                queue.append(track);
            }
            drop(queue);
            ctx.events.state.send_modify(|state| state.error = None);
            handle_play(ctx, state, false);
        }
        PlayerCmd::Next | PlayerCmd::Previous => {
            let next = matches!(cmd, PlayerCmd::Next);
            let mut queue = ctx.queue.lock().unwrap();
            let path = if next { queue.next() } else { queue.previous() };
            drop(queue);
            if path.is_some() {
                handle_play(ctx, state, false);
            } else {
                handle_command(ctx, state, PlayerCmd::Stop);
            }
        }
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
            clear_now_playing(&ctx.events);
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
        PlayerCmd::UpdateControlsMetadata { metadata } => update_controls_metadata(ctx, metadata),
    }
    let queue = ctx.queue.lock().unwrap();
    let info = QueueInfo {
        queue_ids: queue.queue_ids(),
        paths: queue.paths(),
        file_ids: queue.file_ids(),
        occurrences: queue.playlist_ordinals(),
        current_index: queue.index,
        playlist_id: queue.playlist_id,
    };
    drop(queue);
    ctx.events.emit(AudioEvent::QueueChanged(info));
    publish_state(ctx, state);
}

mod transport;
use transport::{clear_now_playing, handle_play};
pub(super) use transport::{fail_playback, handle_track_completion};

mod publish;
pub(super) use publish::publish_track;
use publish::{publish_state, update_controls_metadata};

mod seek;
pub(super) use seek::handle_seek;
