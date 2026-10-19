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
    ToggleShuffle,
    CycleRepeat,
    Load {
        tracks: Vec<crate::queue_track::UnresolvedTrack>,
    },
    LoadAt {
        tracks: Vec<crate::queue_track::UnresolvedTrack>,
        index: usize,
    },
    Enqueue {
        tracks: Vec<crate::queue_track::UnresolvedTrack>,
        next: bool,
    },
    ReconcileLibraryFiles {
        path_updates: std::collections::HashMap<i64, String>,
        file_ids: Vec<i64>,
    },
    RemoveMissingFiles {
        file_ids: Vec<i64>,
    },
    PlayQueueEntry {
        queue_id: String,
    },
    RemoveQueueEntry {
        queue_id: String,
    },
    ReorderQueueEntry {
        queue_id: String,
        target_id: String,
    },
    ClearQueue,
    Next,
    Previous,
    Play {
        resuming: bool,
    },
    Pause,
    Stop,
    Seek {
        seconds: u64,
    },
    SeekMilliseconds {
        milliseconds: u64,
    },
    UpdateControlsMetadata {
        metadata: PartialMetadata,
    },
    UpdateDeviceConfig {
        target_sample_rate: u32,
        output_channels: usize,
    },
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
