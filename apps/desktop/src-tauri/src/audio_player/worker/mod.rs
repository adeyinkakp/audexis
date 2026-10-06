mod commands;
mod decode;
mod state;

use ringbuf::traits::{Observer, Producer};
use rubato::Resampler;
use souvlaki::MediaControls;
use std::sync::atomic::AtomicI64;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tauri::AppHandle;

use super::queue::Queue;
use super::types::PlayerCmd;
use state::WorkerState;

pub struct WorkerContext<P>
where
    P: Producer<Item = f32> + Observer + Send,
{
    pub queue: Arc<Mutex<Queue>>,
    pub play_counter: Arc<Mutex<super::play_count::PlayCounter>>,
    pub cmd_rx: crossbeam_channel::Receiver<PlayerCmd>,
    pub cmd_tx: crossbeam_channel::Sender<PlayerCmd>,
    pub producer: P,
    pub controls: Arc<Mutex<MediaControls>>,
    pub app_handle: AppHandle,
    pub database_id: Arc<AtomicI64>,
    pub audio_duration: Arc<AtomicU64>,
    pub flush_output: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub position_ms: Arc<AtomicU64>,
}

pub fn run<P>(mut ctx: WorkerContext<P>)
where
    P: Producer<Item = f32> + Observer + Send,
{
    let mut state = WorkerState::new();

    loop {
        if state.is_done {
            commands::handle_track_completion(&mut ctx, &mut state);
        }

        while let Ok(cmd) = ctx.cmd_rx.try_recv() {
            commands::handle_command(&mut ctx, &mut state, cmd);
        }

        let idle =
            state.format.is_none() || state.decoder.is_none() || ctx.paused.load(Ordering::Acquire);
        let blocked = !idle && !decode::flush_pending_audio(&mut ctx.producer, &mut state);
        if idle || blocked {
            let timeout = Duration::from_millis(if idle { 10 } else { 2 });
            match ctx.cmd_rx.recv_timeout(timeout) {
                Ok(cmd) => commands::handle_command(&mut ctx, &mut state, cmd),
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            }
            continue;
        }

        if !decode::ensure_resampler(&mut state) {
            continue;
        }

        let required_samples =
            state.resampler.as_mut().unwrap().input_frames_next() * state.channels_uz;

        if state.decode_buffer.len() < required_samples
            && !decode::fill_decode_buffer(&mut ctx, &mut state)
        {
            continue;
        }

        if state.decode_buffer.len() >= required_samples {
            decode::write_resampled_audio(&mut ctx.producer, &mut state, required_samples);
        }
    }
}
