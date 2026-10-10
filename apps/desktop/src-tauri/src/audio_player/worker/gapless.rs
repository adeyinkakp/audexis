use super::{commands, source, state::Transition, WorkerContext, WorkerState};
use crate::commands::playback::shared::QueueInfo;
use ringbuf::traits::{Observer, Producer};
use std::sync::atomic::Ordering;
use tauri::Emitter;

pub fn publish_boundary<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState)
where
    P: Producer<Item = f32> + Observer + Send,
{
    if ctx.flush_output.load(Ordering::Acquire) {
        return;
    }
    while let Some(transition) = state.transitions.front() {
        let mut queue = ctx.queue.lock().unwrap();
        let next = queue.next_after_eof_index();
        let valid = next
            .and_then(|index| queue.tracks.get(index))
            .is_some_and(|track| track.lock().unwrap().queue_id == transition.queue_id);
        if !valid {
            drop(queue);

            let position = ctx.position_ms.load(Ordering::Acquire);
            commands::handle_seek(ctx, state, position);
            return;
        }
        let consumed = ctx.consumed_samples.load(Ordering::Acquire);
        if consumed < transition.boundary {
            return;
        }
        queue.index = next.unwrap() as i32;
        let info = QueueInfo {
            queue_ids: queue.queue_ids(),
            paths: queue.paths(),
            file_ids: queue.file_ids(),
            occurrences: queue.playlist_ordinals(),
            current_index: queue.index,
            playlist_id: queue.playlist_id,
        };
        drop(queue);
        let transition = state.transitions.pop_front().unwrap();
        let position = (consumed - transition.boundary) * 1000
            / (u64::from(state.target_sample_rate) * state.output_channels as u64);
        commands::publish_track(
            ctx,
            transition.database_id,
            transition.duration,
            transition.playlist_id,
            position,
        );
        let _ = ctx.app_handle.emit("queue-changed", info);
        let _ = ctx.app_handle.emit("playback-media-done", ());
    }
}

pub fn finish_decoding<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState)
where
    P: Producer<Item = f32> + Observer + Send,
{
    if state.transitions.len() >= 32 {
        return;
    }
    if !state.next_failed {
        let queue = ctx.queue.lock().unwrap();
        let decoding_index = state
            .transitions
            .back()
            .and_then(|transition| {
                queue
                    .queue_ids()
                    .iter()
                    .position(|id| id == &transition.queue_id)
            })
            .unwrap_or(queue.index as usize);
        let candidate = queue.next_after_index(decoding_index).map(|index| {
            let track = queue.tracks[index].lock().unwrap();
            (
                track.path.clone(),
                track.queue_id.clone(),
                track.database_id,
                queue.playlist_id.filter(|_| track.playlist_ord.is_some()),
            )
        });
        drop(queue);
        if let Some((path, queue_id, database_id, playlist_id)) = candidate {
            match source::open(&path, state.target_sample_rate, state.output_channels) {
                Ok((next, duration)) => {
                    let mut previous = std::mem::replace(state, next);
                    state.transitions = std::mem::take(&mut previous.transitions);
                    state.transitions.push_back(Transition {
                        boundary: ctx.produced_samples,
                        queue_id,
                        database_id,
                        duration,
                        playlist_id,
                        previous: Box::new(previous),
                    });
                    return;
                }
                Err(error) => {
                    tauri_plugin_log::log::error!("Could not prepare next track: {error}");
                    state.next_failed = true;
                }
            }
        }
    }

    if state.transitions.is_empty() && ctx.producer.is_empty() {
        ctx.queue.lock().unwrap().advance_after_eof();
        state.is_done = true;
    }
}
