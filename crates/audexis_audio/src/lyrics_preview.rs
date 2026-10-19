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
    formats::{FormatReader, SeekMode, SeekTo, TrackType, probe::Hint},
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

mod decoder;
mod worker;
use decoder::Decoder;
use worker::run;

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
