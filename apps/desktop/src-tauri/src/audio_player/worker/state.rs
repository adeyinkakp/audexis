use rubato::Async;
use std::collections::VecDeque;
use symphonia::core::codecs::audio::AudioDecoder;
use symphonia::core::formats::FormatReader;

pub struct Transition {
    pub boundary: u64,
    pub queue_id: String,
    pub database_id: i64,
    pub duration: u64,
    pub playlist_id: Option<i64>,
    pub previous: Box<WorkerState>,
}

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
    pub pending_output: std::ops::Range<usize>,
    pub output_channels: usize,
    pub eof: bool,
    pub input_frames: u64,
    pub output_frames: u64,
    pub delay_remaining: usize,
    pub transitions: VecDeque<Transition>,
    pub next_failed: bool,
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
            pending_output: 0..0,
            output_channels: 2,
            eof: false,
            input_frames: 0,
            output_frames: 0,
            delay_remaining: 0,
            transitions: VecDeque::new(),
            next_failed: false,
            decode_buffer: Vec::new(),
            packet_samples: Vec::new(),
        }
    }

    pub fn restore_audible_track(&mut self) {
        if let Some(transition) = self.transitions.pop_front() {
            *self = *transition.previous;
        }
    }

    pub fn clear_playback_buffers(&mut self) {
        self.eof = false;
        self.input_frames = 0;
        self.output_frames = 0;
        self.delay_remaining = 0;
        self.next_failed = false;
        self.pending_output = 0..0;
        self.decode_buffer.clear();
        self.packet_samples.clear();
    }

    pub fn clear_loaded_track(&mut self) {
        self.is_done = false;
        self.transitions.clear();
        self.format = None;
        self.decoder = None;
        self.resampler = None;
        self.clear_playback_buffers();
    }
}
