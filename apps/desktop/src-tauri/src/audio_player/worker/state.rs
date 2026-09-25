use rubato::{Async, Indexing};
use symphonia::core::codecs::audio::AudioDecoder;
use symphonia::core::formats::FormatReader;

pub struct WorkerState {
    pub format: Option<Box<dyn FormatReader>>,
    pub decoder: Option<Box<dyn AudioDecoder>>,
    pub track_id: u32,
    pub source_sample_rate: u32,
    pub channels_uz: usize,
    pub target_sample_rate: u32,
    pub is_done: bool,
    pub resampler: Option<Async<f32>>,
    pub indata: Vec<f32>,
    pub outdata: Vec<f32>,
    pub indexing: Indexing,
    pub decode_buffer: Vec<f32>,
    pub packet_samples: Vec<f32>,
}

impl WorkerState {
    pub fn new() -> Self {
        Self {
            format: None,
            decoder: None,
            track_id: 0,
            source_sample_rate: 44100,
            channels_uz: 2,
            target_sample_rate: 44100,
            is_done: false,
            resampler: None,
            indata: Vec::new(),
            outdata: Vec::new(),
            indexing: Indexing::new(),
            decode_buffer: Vec::new(),
            packet_samples: Vec::new(),
        }
    }

    pub fn clear_playback_buffers(&mut self) {
        self.decode_buffer.clear();
        self.packet_samples.clear();
    }

    pub fn clear_loaded_track(&mut self) {
        self.format = None;
        self.decoder = None;
        self.clear_playback_buffers();
    }
}
