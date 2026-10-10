use audioadapter_buffers::direct::InterleavedSlice;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender};
use rubato::{Async, FixedAsync, PolynomialDegree, Resampler};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use symphonia::core::{
    codecs::audio::AudioDecoder,
    formats::{probe::Hint, FormatReader, SeekMode, SeekTo, TrackType},
    io::MediaSourceStream,
    units::{Time, TimeBase},
};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Open,
    Play,
    Pause,
    Seek,
    Status,
    Close,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub position_ms: u64,
    pub duration_ms: u64,
    pub playing: bool,
    pub error: Option<String>,
}

#[derive(Default)]
struct Output {
    samples: VecDeque<f32>,
    frames: u64,
    start_ms: u64,
    duration_ms: u64,
    playing: bool,
    eof: bool,
    error: Option<String>,
}
impl Output {
    fn status(&self, rate: u32) -> Status {
        Status {
            position_ms: (self.start_ms + self.frames * 1000 / u64::from(rate))
                .min(self.duration_ms),
            duration_ms: self.duration_ms,
            playing: self.playing,
            error: self.error.clone(),
        }
    }
}

type Request = (Action, u64, Sender<Result<Status, String>>);
pub struct Preview {
    pub session_id: String,
    sender: Sender<Request>,
}
impl Preview {
    pub fn open(
        session_id: String,
        path: String,
        equalizer: Arc<Mutex<super::equalizer::Settings>>,
    ) -> Result<(Self, Status), String> {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let (ready_tx, ready_rx) = crossbeam_channel::bounded(1);
        std::thread::spawn(move || {
            if let Err(error) = run(path, receiver, &ready_tx, equalizer) {
                let _ = ready_tx.send(Err(error));
            }
        });
        let preview = Self { session_id, sender };
        let status = ready_rx
            .recv()
            .map_err(|_| "Preview worker stopped".to_string())??;
        Ok((preview, status))
    }
    pub fn request(&self, action: Action, position_ms: u64) -> Result<Status, String> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        self.sender
            .send((action, position_ms, tx))
            .map_err(|_| "Preview worker stopped".to_string())?;
        rx.recv_timeout(Duration::from_secs(10))
            .map_err(|_| "Preview did not respond".to_string())?
    }
}
impl Drop for Preview {
    fn drop(&mut self) {
        let (tx, _) = crossbeam_channel::bounded(1);
        let _ = self.sender.send((Action::Close, 0, tx));
    }
}

struct Decoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    rate: u32,
    channels: usize,
    time_base: Option<TimeBase>,
    skip_until_ms: Option<u64>,
    pending: Vec<f32>,
    eof: bool,
}
impl Decoder {
    fn open(path: &str) -> Result<(Self, u64), String> {
        let file = crate::utils::library_files::open(path).map_err(|e| e.to_string())?;
        let mut hint = Hint::new();
        if let Some(ext) = std::path::Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
        {
            hint.with_extension(ext);
        }
        let mut format = symphonia::default::get_probe()
            .probe(
                &hint,
                MediaSourceStream::new(Box::new(file), Default::default()),
                Default::default(),
                Default::default(),
            )
            .map_err(|e| e.to_string())?;
        let track = format
            .default_track(TrackType::Audio)
            .ok_or("No audio track")?;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or("No audio parameters")?;
        let rate = params
            .sample_rate
            .filter(|r| *r > 0)
            .ok_or("No sample rate")?;
        let channels = params
            .channels
            .as_ref()
            .map(|c| c.count())
            .filter(|c| *c > 0)
            .ok_or("No channels")?;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &Default::default())
            .map_err(|e| e.to_string())?;
        let track_id = track.id;
        let time_base = track.time_base;
        let duration = super::duration::resolve_duration_ms(&mut format, track_id, None).ms;
        Ok((
            Self {
                format,
                decoder,
                track_id,
                rate,
                channels,
                time_base,
                skip_until_ms: None,
                pending: Vec::new(),
                eof: false,
            },
            duration,
        ))
    }
    fn seek(&mut self, ms: u64) -> Result<(), String> {
        self.format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::try_new((ms / 1000) as i64, ((ms % 1000) * 1_000_000) as u32)
                        .ok_or("Invalid position")?,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|e| e.to_string())?;
        self.decoder.reset();
        self.pending.clear();
        self.eof = false;
        self.skip_until_ms = Some(ms);
        Ok(())
    }
    fn fill(&mut self) -> Result<(), String> {
        let Some(packet) = self.format.next_packet().map_err(|e| e.to_string())? else {
            self.eof = true;
            return Ok(());
        };
        if packet.track_id != self.track_id {
            return Ok(());
        }
        let decoded = self.decoder.decode(&packet).map_err(|e| e.to_string())?;
        let mut samples = vec![0.0; decoded.samples_interleaved()];
        decoded.copy_to_slice_interleaved(&mut samples);
        let skip = if let (Some(target), Some(tb)) = (self.skip_until_ms, self.time_base) {
            let packet_frame =
                (i128::from(packet.pts.get()) * i128::from(tb.numer.get()) * i128::from(self.rate))
                    / i128::from(tb.denom.get());
            let target_frame = i128::from(target) * i128::from(self.rate) / 1000;
            (target_frame - packet_frame)
                .max(0)
                .min((samples.len() / self.channels) as i128) as usize
        } else {
            0
        };
        if skip * self.channels < samples.len() {
            self.skip_until_ms = None;
        }
        self.pending
            .extend_from_slice(&samples[skip * self.channels..]);
        Ok(())
    }
}

fn run(
    path: String,
    commands: Receiver<Request>,
    ready: &Sender<Result<Status, String>>,
    equalizer_settings: Arc<Mutex<super::equalizer::Settings>>,
) -> Result<(), String> {
    let (mut decoder, duration) = Decoder::open(&path)?;
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("No audio output device")?;
    let config = device
        .default_output_config()
        .map_err(|e| e.to_string())?
        .config();
    let rate = config.sample_rate;
    let channels = usize::from(config.channels);
    let output = Arc::new(Mutex::new(Output {
        duration_ms: duration,
        ..Output::default()
    }));
    let callback_output = output.clone();
    let error_output = output.clone();
    let initial_eq = *equalizer_settings
        .lock()
        .map_err(|_| "Equalizer unavailable")?;
    let mut equalizer = super::equalizer::Equalizer::new(rate, channels, initial_eq);
    let mut last_start_ms = 0;
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| {
                data.fill(0.0);
                if let Ok(mut state) = callback_output.try_lock() {
                    if !state.playing {
                        return;
                    }
                    let frames = (data.len() / channels).min(state.samples.len() / channels);
                    for sample in &mut data[..frames * channels] {
                        *sample = state.samples.pop_front().unwrap_or(0.0);
                    }
                    if state.start_ms != last_start_ms || state.frames == 0 {
                        equalizer.reset();
                        last_start_ms = state.start_ms;
                    }
                    if let Ok(settings) = equalizer_settings.try_lock() {
                        equalizer.update(*settings);
                    }
                    equalizer.process(&mut data[..frames * channels]);
                    state.frames += frames as u64;
                    if state.eof && state.samples.is_empty() {
                        state.playing = false;
                    }
                }
            },
            move |error| {
                if let Ok(mut state) = error_output.lock() {
                    state.error = Some(error.to_string());
                    state.playing = false;
                }
            },
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    let mut resampler = Async::<f32>::new_poly(
        rate as f64 / decoder.rate as f64,
        1.1,
        PolynomialDegree::Cubic,
        1024,
        decoder.channels,
        FixedAsync::Input,
    )
    .map_err(|e| e.to_string())?;
    let _ = ready.send(Ok(output
        .lock()
        .map_err(|_| "Preview unavailable")?
        .status(rate)));
    loop {
        let waiting = {
            let state = output.lock().map_err(|_| "Preview unavailable")?;
            !state.playing
                || state.eof
                || state.error.is_some()
                || state.samples.len() >= 8192 * channels
        };
        match commands.recv_timeout(Duration::from_millis(if waiting { 10 } else { 0 })) {
            Ok((action, ms, reply)) => {
                if matches!(action, Action::Close) {
                    let _ = reply.send(Ok(Status::default()));
                    return Ok(());
                }
                let result = (|| {
                    let mut state = output
                        .lock()
                        .map_err(|_| "Preview unavailable".to_string())?;
                    match action {
                        Action::Play => {
                            if state.eof && state.samples.is_empty() {
                                decoder.seek(0)?;
                                resampler.reset();
                                state.samples.clear();
                                state.frames = 0;
                                state.start_ms = 0;
                                state.eof = false;
                            }
                            state.playing = true;
                        }
                        Action::Pause => state.playing = false,
                        Action::Seek => {
                            let target = ms.min(duration);
                            decoder.seek(target)?;
                            resampler.reset();
                            state.samples.clear();
                            state.frames = 0;
                            state.start_ms = target;
                            state.eof = false;
                        }
                        _ => {}
                    }
                    Ok(state.status(rate))
                })();
                let _ = reply.send(result);
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return Ok(()),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
        }
        {
            let state = output.lock().map_err(|_| "Preview unavailable")?;
            if !state.playing
                || state.error.is_some()
                || state.samples.len() >= 8192 * channels
                || state.eof
            {
                continue;
            }
        }
        let required = resampler.input_frames_next() * decoder.channels;
        if decoder.pending.len() < required && !decoder.eof {
            if let Err(error) = decoder.fill() {
                let mut state = output.lock().map_err(|_| "Preview unavailable")?;
                state.error = Some(error);
                state.playing = false;
            }
            continue;
        }
        if decoder.pending.is_empty() && decoder.eof {
            output.lock().map_err(|_| "Preview unavailable")?.eof = true;
            continue;
        }
        let take = required.min(decoder.pending.len());
        let mut input: Vec<f32> = decoder.pending.drain(..take).collect();
        input.resize(required, 0.0);
        let mut samples = vec![0.0; resampler.output_frames_max() * decoder.channels];
        let capacity = samples.len() / decoder.channels;
        let (_, written) = resampler
            .process_into_buffer(
                &InterleavedSlice::new(&input, decoder.channels, required / decoder.channels)
                    .map_err(|e| e.to_string())?,
                &mut InterleavedSlice::new_mut(&mut samples, decoder.channels, capacity)
                    .map_err(|e| e.to_string())?,
                None,
            )
            .map_err(|e| e.to_string())?;
        let mut state = output.lock().map_err(|_| "Preview unavailable")?;
        for frame in samples[..written * decoder.channels].chunks_exact(decoder.channels) {
            for channel in 0..channels {
                state.samples.push_back(if channels == 1 {
                    frame.iter().sum::<f32>() / frame.len() as f32
                } else if decoder.channels == 1 {
                    frame[0]
                } else {
                    frame.get(channel).copied().unwrap_or(0.0)
                });
            }
        }
    }
}
// shout out chat gtt
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_uses_total_frames_without_per_callback_rounding_drift() {
        let output = Output {
            frames: 44100,
            start_ms: 1250,
            duration_ms: 10000,
            ..Output::default()
        };
        assert_eq!(output.status(44100).position_ms, 2250);
    }
}

#[cfg(test)]
mod decoding_tests {
    use super::*;

    #[test]
    fn native_decoder_seeks_to_a_fractional_second_without_audio_device() {
        let path =
            std::env::temp_dir().join(format!("audexis-preview-{}.wav", uuid::Uuid::new_v4()));
        let rate = 8000u32;
        let data_size = rate * 2 * 2;
        let mut bytes = b"RIFF".to_vec();
        bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt \x10\0\0\0\x01\0\x01\0");
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&(rate * 2).to_le_bytes());
        bytes.extend_from_slice(b"\x02\0\x10\0data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        for frame in 0..rate * 2 {
            bytes.extend_from_slice(&((frame % 997) as i16).to_le_bytes());
        }
        std::fs::write(&path, bytes).unwrap();
        let (mut decoder, duration) = Decoder::open(path.to_str().unwrap()).unwrap();
        assert_eq!(duration, 2000);
        decoder.seek(1250).unwrap();
        while decoder.pending.is_empty() && !decoder.eof {
            decoder.fill().unwrap();
        }
        let expected = ((10000 % 997) as f32) / 32768.0;
        assert!((decoder.pending[0] - expected).abs() < 0.00001);
        decoder.seek(250).unwrap();
        while decoder.pending.is_empty() && !decoder.eof {
            decoder.fill().unwrap();
        }
        assert!((decoder.pending[0] - (2000 % 997) as f32 / 32768.0).abs() < 0.00001);
        drop(decoder);
        std::fs::remove_file(path).unwrap();
    }
}
