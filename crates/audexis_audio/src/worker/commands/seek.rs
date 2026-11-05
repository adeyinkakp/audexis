use super::*;

pub(in crate::worker) fn handle_seek<P>(
    ctx: &mut WorkerContext<P>,
    state: &mut WorkerState,
    milliseconds: u64,
) where
    P: Producer<Item = f32> + Observer + Send,
{
    if let Err(error) = crate::decoder_guard::guard(|| {
        seek_inner(ctx, state, milliseconds);
        Ok(())
    }) {
        fail_playback(ctx, state, &error);
    }
}

pub(in crate::worker) fn seek_inner<P>(
    ctx: &mut WorkerContext<P>,
    state: &mut WorkerState,
    milliseconds: u64,
) where
    P: Producer<Item = f32> + Observer + Send,
{
    let duration = ctx.audio_duration.load(Ordering::Acquire);
    let milliseconds = if duration > 0 {
        milliseconds.min(duration)
    } else {
        milliseconds
    };
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
            tracing::error!("Could not seek playback: {error}");
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
