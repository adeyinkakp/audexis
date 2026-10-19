use audioadapter_buffers::direct::InterleavedSlice;
use ringbuf::traits::{Observer, Producer};
use rubato::{Async, FixedAsync, PolynomialDegree, Resampler};
use symphonia::core::errors::Error;

use super::{WorkerContext, WorkerState};

pub fn render(state: &mut WorkerState) -> Result<bool, String> {
    let resampling = state.source_sample_rate != state.target_sample_rate;
    if resampling && state.resampler.is_none() {
        let resampler = Async::<f32>::new_poly(
            state.target_sample_rate as f64 / state.source_sample_rate as f64,
            1.1,
            PolynomialDegree::Cubic,
            1024,
            state.channels_uz,
            FixedAsync::Input,
        )
        .map_err(|e| e.to_string())?;
        state.delay_remaining = resampler.output_delay();
        state.resampler = Some(resampler);
    }
    let required = state
        .resampler
        .as_mut()
        .map_or(1024, |r| r.input_frames_next());
    while !state.eof && state.decode_buffer.len() < required * state.channels_uz {
        match state.format.as_mut().ok_or("No format")?.next_packet() {
            Ok(Some(packet)) => {
                if packet.track_id != state.track_id {
                    continue;
                }
                match state.decoder.as_mut().ok_or("No decoder")?.decode(&packet) {
                    Ok(audio) => {
                        state
                            .packet_samples
                            .resize(audio.samples_interleaved(), 0.0);
                        audio.copy_to_slice_interleaved(&mut state.packet_samples);
                        state.input_frames +=
                            (state.packet_samples.len() / state.channels_uz) as u64;
                        state.decode_buffer.extend_from_slice(&state.packet_samples);
                    }
                    Err(Error::DecodeError(_)) => continue,
                    Err(error) => return Err(error.to_string()),
                }
            }
            Ok(None) => state.eof = true,
            Err(Error::IoError(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                state.eof = true
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    let expected = (u128::from(state.input_frames) * u128::from(state.target_sample_rate))
        .div_ceil(u128::from(state.source_sample_rate)) as u64;
    if state.eof && state.output_frames >= expected {
        return Ok(false);
    }
    let available = (state.decode_buffer.len() / state.channels_uz).min(required);
    state.indata.clear();
    state
        .indata
        .extend(state.decode_buffer.drain(..available * state.channels_uz));
    let (samples, frames) = if let Some(resampler) = state.resampler.as_mut() {
        state.indata.resize(required * state.channels_uz, 0.0);
        let capacity = resampler.output_frames_max();
        let mut output = vec![0.0; capacity * state.channels_uz];
        let input = InterleavedSlice::new(&state.indata, state.channels_uz, required)
            .map_err(|e| e.to_string())?;
        let mut adapter = InterleavedSlice::new_mut(&mut output, state.channels_uz, capacity)
            .map_err(|e| e.to_string())?;
        let (_, written) = resampler
            .process_into_buffer(&input, &mut adapter, None)
            .map_err(|e| e.to_string())?;
        let skip = state.delay_remaining.min(written);
        state.delay_remaining -= skip;
        let keep = if state.eof {
            (written - skip).min(expected.saturating_sub(state.output_frames) as usize)
        } else {
            written - skip
        };
        (
            output[skip * state.channels_uz..(skip + keep) * state.channels_uz].to_vec(),
            keep,
        )
    } else {
        (state.indata.clone(), available)
    };
    state.outdata.clear();
    for frame in samples.chunks_exact(state.channels_uz) {
        if state.output_channels == 1 {
            state
                .outdata
                .push(frame.iter().sum::<f32>() / state.channels_uz as f32);
        } else {
            for channel in 0..state.output_channels {
                state.outdata.push(if state.channels_uz == 1 {
                    frame[0]
                } else {
                    frame.get(channel).copied().unwrap_or(0.0)
                });
            }
        }
    }
    state.output_frames += frames as u64;
    state.pending_output = 0..state.outdata.len();
    Ok(true)
}

pub fn flush_pending_audio<P>(ctx: &mut WorkerContext<P>, state: &mut WorkerState) -> bool
where
    P: Producer<Item = f32> + Observer + Send,
{
    let limit = super::output_buffer_samples(state.target_sample_rate, state.output_channels);
    let written = push_with_limit(
        &mut ctx.producer,
        &state.outdata[state.pending_output.clone()],
        limit,
    );
    ctx.produced_samples += written as u64;
    state.pending_output.start += written;
    state.pending_output.is_empty()
}

fn push_with_limit<P: Producer<Item = f32> + Observer>(
    producer: &mut P,
    samples: &[f32],
    limit: usize,
) -> usize {
    let count = samples
        .len()
        .min(limit.saturating_sub(producer.occupied_len()));
    producer.push_slice(&samples[..count])
}
