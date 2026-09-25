use audioadapter_buffers::direct::InterleavedSlice;
use ringbuf::traits::{Observer, Producer};
use rubato::{Async, FixedAsync, PolynomialDegree, Resampler};

use std::time::Duration;
use symphonia::core::errors::Error;

use super::{WorkerContext, WorkerState};
use crate::audio_player::types::AudioPlayerError;

pub fn ensure_resampler(state: &mut WorkerState) -> bool {
    if state.resampler.is_some() {
        return true;
    }

    let resample_ratio = state.target_sample_rate as f64 / state.source_sample_rate as f64;
    let chunk_size = 1024usize;
    let resampler = match Async::<f32>::new_poly(
        resample_ratio,
        1.1,
        PolynomialDegree::Cubic,
        chunk_size,
        state.channels_uz,
        FixedAsync::Input,
    ) {
        Ok(resampler) => resampler,
        Err(error) => {
            tauri_plugin_log::log::error!(
                "{}",
                AudioPlayerError::Resampler {
                    message: error.to_string(),
                }
            );
            state.resampler = None;
            return false;
        }
    };

    state.indata = vec![0.0f32; state.channels_uz * chunk_size];
    state.outdata = vec![0.0f32; state.channels_uz * resampler.output_frames_max()];
    state.resampler = Some(resampler);
    true
}

pub fn fill_decode_buffer<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState) -> bool
where
    P: Producer<Item = f32> + Observer + Send,
{
    let fmt_ref = state.format.as_mut().unwrap();
    let dec_ref = state.decoder.as_mut().unwrap();

    match fmt_ref.next_packet() {
        Ok(packet) => {
            if packet.is_none() {
                return handle_end_of_stream(ctx, state);
            }
            let packet = packet.unwrap();
            if packet.track_id == state.track_id {
                if let Ok(audio_buf) = dec_ref.decode(&packet) {
                    let needed = audio_buf.samples_interleaved();
                    state.packet_samples.resize(needed, 0.0);
                    audio_buf.copy_to_slice_interleaved(&mut state.packet_samples);
                    state.decode_buffer.extend_from_slice(&state.packet_samples);
                }
            }
            true
        }
        Err(Error::ResetRequired) => {
            println!("hmm");
            true
        }
        Err(_) => {
            println!("eof");
            handle_end_of_stream(ctx, state)
        }
    }
}

pub fn handle_end_of_stream<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState) -> bool
where
    P: Producer<Item = f32> + Observer + Send,
{
    if !ctx.producer.is_empty() {
        std::thread::sleep(Duration::from_millis(10));
        return false;
    }

    if state.is_done {
        return false;
    }

    let mut q = ctx.queue.lock().unwrap();
    let has_next = q.advance_after_eof();
    drop(q);

    state.is_done = true;
    if has_next {
        let _ = ctx
            .cmd_tx
            .send(crate::audio_player::PlayerCmd::Play { resuming: false });
    }

    false
}

pub fn write_resampled_audio<P>(
    ctx: &mut WorkerContext<P>,
    state: &mut WorkerState,
    required_samples: usize,
) where
    P: Producer<Item = f32> + Observer + Send,
{
    let chunk_samples: Vec<f32> = state.decode_buffer.drain(0..required_samples).collect();
    state.indata.copy_from_slice(&chunk_samples);

    let current_resampler = state.resampler.as_mut().unwrap();
    let frames_to_read = current_resampler.input_frames_next();
    let input_adapter =
        InterleavedSlice::new(&state.indata, state.channels_uz, frames_to_read).unwrap();

    let current_capacity = state.outdata.len() / state.channels_uz;
    let mut output_adapter =
        InterleavedSlice::new_mut(&mut state.outdata, state.channels_uz, current_capacity).unwrap();

    let (_, frames_written) = current_resampler
        .process_into_buffer(&input_adapter, &mut output_adapter, Some(&state.indexing))
        .unwrap();

    let total_written = frames_written * state.channels_uz;
    let resampled_slice = &state.outdata[0..total_written];

    let mut written = 0usize;
    while written < resampled_slice.len() {
        let pushed = ctx.producer.push_slice(&resampled_slice[written..]);
        written += pushed;
        if written < resampled_slice.len() {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
