use std::fmt;

use rubato::{Async, Indexing};
use serde::{Deserialize, Serialize};
use symphonia::core::codecs::audio::AudioDecoder;
use symphonia::core::formats::FormatReader;
use uuid::Uuid;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnresolvedTrack {
    pub id: i64,
    pub path: String,
    pub occurrence: Option<i64>,
}
pub struct QueueTrack {
    pub queue_id: String,
    pub playlist_ord: Option<i64>,
    pub format: Option<Box<dyn FormatReader>>,
    pub decoder: Option<Box<dyn AudioDecoder>>,
    pub track_id: u32,
    pub database_id: i64,
    pub source_sample_rate: u32,
    pub channels_uz: usize,
    pub target_sample_rate: u32,
    pub is_preloaded: bool,
    pub resampler: Option<Async<f32>>,
    pub indata: Vec<f32>,
    pub outdata: Vec<f32>,
    pub indexing: Indexing,
    pub path: String,
    pub decode_buffer: Vec<f32>,
    pub packet_samples: Vec<f32>,
    pub duration_ms: Option<u64>,
    pub duration_exact: bool,
}
impl fmt::Debug for QueueTrack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueeTrack")
            .field("path", &self.path)
            .finish()
    }
}
impl QueueTrack {
    pub fn new(unresolved_track: UnresolvedTrack) -> Self {
        Self {
            queue_id: Uuid::new_v4().to_string(),
            playlist_ord: unresolved_track.occurrence,
            format: None,
            decoder: None,
            is_preloaded: false,
            track_id: 0,
            source_sample_rate: 44100,
            channels_uz: 2usize,
            target_sample_rate: 44100u32,
            path: unresolved_track.path,
            database_id: unresolved_track.id as i64,
            resampler: None,
            indata: Vec::new(),
            outdata: Vec::new(),
            indexing: Indexing::new(),

            decode_buffer: Vec::new(),
            packet_samples: Vec::new(),
            duration_ms: None,
            duration_exact: true,
        }
    }
    pub fn unload(&mut self) {
        self.format = None;
        self.decoder = None;
        self.is_preloaded = false;
        self.track_id = 0;
        self.source_sample_rate = 44100;
        self.channels_uz = 2usize;
        self.target_sample_rate = 44100u32;

        self.resampler = None;
        self.indata = Vec::new();
        self.outdata = Vec::new();
        self.indexing = Indexing::new();

        self.decode_buffer = Vec::new();
        self.packet_samples = Vec::new();
    }
    pub fn get_indexing(&self) -> &Indexing {
        return &self.indexing;
    }
}
