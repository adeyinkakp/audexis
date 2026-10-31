mod commands;
pub(crate) mod decode;
mod gapless;
pub(crate) mod source;
mod state;

use crate::events::EventBus;
use crate::media::MediaSession;
use ringbuf::traits::{Observer, Producer};
use std::sync::atomic::AtomicI64;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use super::queue::Queue;
use super::types::PlayerCmd;
use state::WorkerState;

pub(super) fn output_buffer_samples(sample_rate: u32, channels: usize) -> usize {
    (sample_rate as usize)
        .saturating_mul(channels)
        .saturating_mul(2)
        .max(1)
}

pub struct WorkerContext<P>
where
    P: Producer<Item = f32> + Observer + Send,
{
    pub queue: Arc<Mutex<Queue>>,
    pub play_counter: Arc<Mutex<super::play_count::PlayCounter>>,
    pub cmd_rx: crossbeam_channel::Receiver<PlayerCmd>,
    pub cmd_tx: crossbeam_channel::Sender<PlayerCmd>,
    pub producer: P,
    pub controls: Arc<Mutex<MediaSession>>,
    pub events: EventBus,
    pub shutdown: Arc<AtomicBool>,
    pub database_id: Arc<AtomicI64>,
    pub audio_duration: Arc<AtomicU64>,
    pub flush_output: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub position_ms: Arc<AtomicU64>,
    pub consumed_samples: Arc<AtomicU64>,
    pub produced_samples: u64,
    pub heard_us: Arc<AtomicU64>,
}

pub fn run<P>(mut ctx: WorkerContext<P>)
where
    P: Producer<Item = f32> + Observer + Send,
{
    let mut state = WorkerState::new();

    while !ctx.shutdown.load(Ordering::Acquire) {
        let heard = ctx.heard_us.swap(0, Ordering::AcqRel);
        if heard > 0
            && let Ok(mut counter) = ctx.play_counter.lock()
        {
            counter.heard(heard);
        }
        while let Ok(cmd) = ctx.cmd_rx.try_recv() {
            commands::handle_command(&mut ctx, &mut state, cmd);
        }

        if state.is_done {
            commands::handle_track_completion(&mut ctx, &mut state);
        }
        gapless::publish_boundary(&mut ctx, &mut state);
        let idle = state.format.is_none()
            || state.decoder.is_none()
            || ctx.paused.load(Ordering::Acquire)
            || ctx.flush_output.load(Ordering::Acquire);
        let blocked = !idle && !decode::flush_pending_audio(&mut ctx, &mut state);
        if idle || blocked {
            let timeout = Duration::from_millis(if idle { 10 } else { 2 });
            match ctx.cmd_rx.recv_timeout(timeout) {
                Ok(cmd) => commands::handle_command(&mut ctx, &mut state, cmd),
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            }
            continue;
        }
        match super::decoder_guard::guard(|| decode::render(&mut state)) {
            Ok(true) => {
                decode::flush_pending_audio(&mut ctx, &mut state);
            }
            Ok(false) => {
                gapless::finish_decoding(&mut ctx, &mut state);
                if state.eof && !state.is_done {
                    match ctx.cmd_rx.recv_timeout(Duration::from_millis(2)) {
                        Ok(cmd) => commands::handle_command(&mut ctx, &mut state, cmd),
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                    }
                }
            }
            Err(error) => {
                commands::fail_playback(&mut ctx, &mut state, &error);
            }
        }
    }
}

#[cfg(test)]
mod tests;
