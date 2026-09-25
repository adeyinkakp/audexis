use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Clone)]
pub struct PartialMetadata {
    pub title: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub cover_url: Option<String>,
    pub duration: Option<u64>,
}

pub enum PlayerCmd {
    Play { resuming: bool },
    Pause,
    Stop,
    Seek { seconds: u64 },
    Preload,
    UpdateControlsMetadata { metadata: PartialMetadata },
    UpdateDeviceConfig { target_sample_rate: u32 },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NowPlayingInfo {
    pub current_position: u64,
    pub paused: bool,
    pub duration: u64,
    pub id: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PartialPlaybackInfo {
    pub id: i64,
    pub duration: u64,
}

#[derive(Debug, Error)]
pub enum AudioPlayerError {
    #[error("failed to open audio file {path}: {source}")]
    FileOpen {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to probe audio file {path}: {message}")]
    Probe { path: String, message: String },
    #[error("audio file {path} has no usable audio track")]
    MissingTrack { path: String },
    #[error("audio file {path} has no sample rate")]
    MissingSampleRate { path: String },
    #[error("audio file {path} has no channel information")]
    MissingChannels { path: String },
    #[error("failed to create decoder for {path}: {message}")]
    Decoder { path: String, message: String },
    #[error("failed to create audio resampler: {message}")]
    Resampler { message: String },
    #[error("no output audio device is available")]
    NoOutputDevice,
    #[error("failed to read output device configuration: {message}")]
    DeviceConfig { message: String },
    #[error("failed to create output stream: {message}")]
    OutputStream { message: String },
    #[error("failed to start output stream: {message}")]
    StartStream { message: String },
}
