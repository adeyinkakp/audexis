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
mod decoder;
mod worker;
use decoder::Decoder;
use worker::run;

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
