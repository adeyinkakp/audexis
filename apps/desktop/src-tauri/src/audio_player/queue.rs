use std::sync::{Arc, Mutex};

use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSourceStream;
use uuid::Uuid;

use crate::audio_player::{
    modes::{PlaybackModes, RepeatMode},
    queue_track::{QueueTrack, UnresolvedTrack},
    AudioPlayerError, PlayerCmd,
};

pub struct Queue {
    pub tracks: Vec<Arc<Mutex<QueueTrack>>>,
    pub index: i32,
    pub repeat_mode: RepeatMode,
    pub shuffled: bool,
    pub playlist_id: Option<i64>,
    original_order: Vec<String>,
    original_queue_ids: Vec<String>,
    shuffle_seed: Option<usize>,
    cmd_tx: crossbeam_channel::Sender<PlayerCmd>,
}

impl Queue {
    pub fn new(cmd_tx: crossbeam_channel::Sender<PlayerCmd>) -> Self {
        Queue {
            tracks: Vec::new(),
            cmd_tx,
            index: 0,
            repeat_mode: RepeatMode::Off,
            shuffled: false,
            playlist_id: None,
            original_order: Vec::new(),
            original_queue_ids: Vec::new(),
            shuffle_seed: None,
        }
    }

    pub fn add(&mut self, item: UnresolvedTrack) -> Result<(), AudioPlayerError> {
        let track = Arc::new(Mutex::new(QueueTrack::new(item.clone())));
        let queue_id = track
            .lock()
            .ok()
            .map(|track| track.queue_id.clone())
            .unwrap_or_default();
        self.tracks.push(track);
        self.original_order.push(item.path);
        self.original_queue_ids.push(queue_id);
        if self.tracks.len() == 1 {
            let _ = self.cmd_tx.send(PlayerCmd::Play { resuming: false });
        }
        self.preload()
    }

    pub fn insert_next(&mut self, item: UnresolvedTrack) {
        let current = self.current_queue_id();
        let original_index = current
            .as_ref()
            .and_then(|id| self.original_queue_ids.iter().position(|value| value == id))
            .map(|i| i + 1)
            .unwrap_or(self.original_queue_ids.len());
        let index = if self.tracks.is_empty() {
            0
        } else {
            (self.index.max(0) as usize + 1).min(self.tracks.len())
        };
        let path = item.path.clone();
        let track = QueueTrack::new(item);
        self.original_queue_ids
            .insert(original_index, track.queue_id.clone());
        self.original_order.insert(original_index, path);
        self.tracks.insert(index, Arc::new(Mutex::new(track)));
    }

    pub fn append(&mut self, item: UnresolvedTrack) {
        self.original_order.push(item.path.clone());
        let track = Arc::new(Mutex::new(QueueTrack::new(item)));
        if let Ok(track_ref) = track.lock() {
            self.original_queue_ids.push(track_ref.queue_id.clone());
        }
        self.tracks.push(track);
    }

    pub fn append_for_playlist(&mut self, playlist_id: i64, item: UnresolvedTrack) -> bool {
        if self.playlist_id != Some(playlist_id) || self.current_queue_id().is_none() {
            return false;
        }
        self.append(item);
        true
    }

    pub fn remove_for_playlist(&mut self, playlist_id: i64, ord: i64) -> bool {
        if self.playlist_id != Some(playlist_id) {
            return false;
        }
        let position = self.tracks.iter().position(|track| {
            track
                .lock()
                .ok()
                .is_some_and(|track| track.playlist_ord == Some(ord))
        });
        let mut removed_current = false;
        if let Some(position) = position {
            let removed = self.tracks.remove(position);
            if let Ok(track) = removed.lock() {
                if let Some(original_index) = self
                    .original_queue_ids
                    .iter()
                    .position(|id| id == &track.queue_id)
                {
                    self.original_queue_ids.remove(original_index);
                    self.original_order.remove(original_index);
                }
            }
            removed_current = position as i32 == self.index;
            if (position as i32) < self.index {
                self.index -= 1;
            }
        }
        for track in &self.tracks {
            if let Ok(mut track) = track.lock() {
                if let Some(position) = track.playlist_ord {
                    if position > ord {
                        track.playlist_ord = Some(position - 1);
                    }
                }
            }
        }
        removed_current
    }

    pub fn reorder_playlist_ordinals(&mut self, playlist_id: i64, from: i64, to: i64) {
        if self.playlist_id != Some(playlist_id) {
            return;
        }
        for track in &self.tracks {
            if let Ok(mut track) = track.lock() {
                track.playlist_ord = track.playlist_ord.map(|ord| {
                    if ord == from {
                        to
                    } else if from < to && ord > from && ord <= to {
                        ord - 1
                    } else if from > to && ord >= to && ord < from {
                        ord + 1
                    } else {
                        ord
                    }
                });
            }
        }
    }

    pub fn file_ids(&self) -> Vec<i64> {
        self.tracks
            .iter()
            .filter_map(|track| track.lock().ok().map(|track| track.database_id))
            .collect()
    }

    pub fn playlist_ordinals(&self) -> Vec<Option<i64>> {
        self.tracks
            .iter()
            .filter_map(|track| track.lock().ok().map(|track| track.playlist_ord))
            .collect()
    }

    pub fn replace(&mut self, item: UnresolvedTrack) -> Result<(), AudioPlayerError> {
        self.playlist_id = None;
        self.unload_all_tracks();
        self.index = 0;
        self.original_order.clear();
        self.original_queue_ids.clear();
        self.shuffle_seed = None;
        self.append(item);
        self.reset_modes();
        Ok(())
    }

    pub fn replace_with(&mut self, items: Vec<UnresolvedTrack>, index: usize) {
        self.clear();
        for item in items {
            self.append(item);
        }
        self.index = index.min(self.tracks.len().saturating_sub(1)) as i32;
    }

    pub fn clear(&mut self) {
        self.playlist_id = None;
        self.unload_all_tracks();
        self.index = 0;
        self.original_order.clear();
        self.original_queue_ids.clear();
        self.shuffle_seed = None;
        self.reset_modes();
    }

    pub fn reset_modes(&mut self) {
        self.repeat_mode = RepeatMode::Off;
        self.shuffled = false;
    }

    pub fn playback_modes(&self) -> PlaybackModes {
        PlaybackModes {
            repeat_mode: self.repeat_mode,
            shuffled: self.shuffled,
        }
    }

    pub fn set_repeat_mode(&mut self, repeat_mode: RepeatMode) {
        self.repeat_mode = repeat_mode;
    }

    pub fn toggle_shuffle(&mut self) {
        if self.tracks.is_empty() {
            self.shuffled = !self.shuffled;
            return;
        }

        let current_queue_id = self.current_queue_id();

        if self.shuffled {
            self.shuffled = false;
            self.shuffle_seed = None;
            self.restore_original_order(current_queue_id.as_deref());
            return;
        }

        self.shuffled = true;
        self.shuffle_seed = Some(generate_shuffle_seed());
        self.shuffle_tracks(current_queue_id.as_deref());
    }

    pub fn next(&mut self) -> Option<String> {
        if self.tracks.is_empty() {
            return None;
        }

        let is_last_track = self.index as usize + 1 >= self.tracks.len();

        if is_last_track {
            return match self.repeat_mode {
                RepeatMode::Queue => {
                    self.index = 0;
                    self.current_path()
                }
                RepeatMode::Track | RepeatMode::Off => {
                    self.index = self.tracks.len() as i32;
                    None
                }
            };
        }

        self.index += 1;
        self.current_path()
    }

    pub fn advance_after_eof(&mut self) -> bool {
        if self.tracks.is_empty() {
            return false;
        }

        match self.repeat_mode {
            RepeatMode::Track => true,
            RepeatMode::Queue => {
                let len = self.tracks.len() as i32;
                self.index = (self.index + 1).rem_euclid(len);
                true
            }
            RepeatMode::Off => {
                let next_index = self.index as usize + 1;
                if next_index < self.tracks.len() {
                    self.index += 1;
                    true
                } else {
                    self.index = self.tracks.len() as i32;
                    false
                }
            }
        }
    }

    pub fn previous(&mut self) -> Option<String> {
        if self.tracks.is_empty() {
            return None;
        }

        if self.index <= 0 {
            if self.repeat_mode == RepeatMode::Queue {
                self.index = self.tracks.len().saturating_sub(1) as i32;
                return self.current_path();
            }
            return None;
        }

        self.index -= 1;
        self.current_path()
    }

    pub fn jump_to_path(&mut self, path: &str) -> bool {
        if let Some(index) = self.tracks.iter().position(|track| {
            track.lock().ok().as_ref().map(|track| track.path.as_str()) == Some(path)
        }) {
            self.index = index as i32;
            return true;
        }

        false
    }

    pub fn current_queue_id(&self) -> Option<String> {
        self.tracks
            .get(self.index as usize)
            .and_then(|track| track.lock().ok().map(|track| track.queue_id.clone()))
    }

    pub fn current_path(&self) -> Option<String> {
        self.tracks
            .get(self.index as usize)
            .and_then(|track| track.lock().ok().map(|track| track.path.clone()))
    }

    pub fn paths(&self) -> Vec<String> {
        self.tracks
            .iter()
            .filter_map(|track| track.lock().ok().map(|track| track.path.clone()))
            .collect()
    }

    pub fn preload(&mut self) -> Result<(), AudioPlayerError> {
        let next_index = self.index as usize + 1;
        let track_to_preload = self.tracks.get(next_index);

        if track_to_preload.is_none() {
            return Ok(());
        }

        let mut track_to_preload = track_to_preload
            .ok_or_else(|| AudioPlayerError::MissingTrack {
                path: "queue".to_string(),
            })?
            .lock()
            .map_err(|_| AudioPlayerError::MissingTrack {
                path: "queue".to_string(),
            })?;
        let file_path = track_to_preload.path.clone();

        let src = std::fs::File::open(&file_path).map_err(|source| AudioPlayerError::FileOpen {
            path: file_path.clone(),
            source,
        })?;

        let mss = MediaSourceStream::new(Box::new(src), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = std::path::Path::new(&file_path)
            .extension()
            .and_then(|e| e.to_str())
        {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .probe(&hint, mss, Default::default(), Default::default())
            .map_err(|error| AudioPlayerError::Probe {
                path: file_path.clone(),
                message: error.to_string(),
            })?;

        let track = probed
            .default_track(symphonia::core::formats::TrackType::Audio)
            .ok_or_else(|| AudioPlayerError::MissingTrack {
                path: file_path.clone(),
            })?;

        track_to_preload.track_id = track.id;

        let audio_codec_params = track
            .codec_params
            .as_ref()
            .ok_or_else(|| AudioPlayerError::MissingTrack {
                path: file_path.clone(),
            })?
            .audio()
            .ok_or_else(|| AudioPlayerError::MissingTrack {
                path: file_path.clone(),
            })?;

        track_to_preload.source_sample_rate =
            audio_codec_params
                .sample_rate
                .ok_or_else(|| AudioPlayerError::MissingSampleRate {
                    path: file_path.clone(),
                })?;
        track_to_preload.channels_uz = audio_codec_params
            .channels
            .as_ref()
            .ok_or_else(|| AudioPlayerError::MissingChannels {
                path: file_path.clone(),
            })?
            .count();

        track_to_preload.decoder = Some(
            symphonia::default::get_codecs()
                .make_audio_decoder(audio_codec_params, &Default::default())
                .map_err(|error| AudioPlayerError::Decoder {
                    path: file_path.clone(),
                    message: error.to_string(),
                })?,
        );

        track_to_preload.format = Some(probed);
        track_to_preload.resampler = None;
        track_to_preload.is_preloaded = true;
        Ok(())
    }

    fn unload_all_tracks(&mut self) {
        for track in self.tracks.drain(..) {
            if let Ok(mut track) = track.lock() {
                track.unload();
            }
        }
    }

    fn restore_original_order(&mut self, current_queue_id: Option<&str>) {
        let mut restored = Vec::with_capacity(self.tracks.len());

        for queue_id in &self.original_queue_ids {
            if let Some(track) = self.tracks.iter().find(|track| {
                track
                    .lock()
                    .ok()
                    .as_ref()
                    .map(|track| track.queue_id.as_str())
                    == Some(queue_id.as_str())
            }) {
                restored.push(Arc::clone(track));
            }
        }

        if restored.len() == self.tracks.len() {
            self.tracks = restored;
        }

        self.index = current_queue_id
            .and_then(|queue_id| {
                self.tracks.iter().position(|track| {
                    track
                        .lock()
                        .ok()
                        .as_ref()
                        .map(|track| track.queue_id.as_str())
                        == Some(queue_id)
                })
            })
            .unwrap_or(0) as i32;
    }

    fn shuffle_tracks(&mut self, current_queue_id: Option<&str>) {
        let current_index = current_queue_id.and_then(|id| {
            self.tracks
                .iter()
                .position(|track| track.lock().ok().is_some_and(|track| track.queue_id == id))
        });
        let current_track = current_index.map(|index| self.tracks.remove(index));
        let seed = self.shuffle_seed.unwrap_or_else(generate_shuffle_seed);
        sort_tracks_by_seed(&mut self.tracks, seed);
        if let Some(track) = current_track {
            self.tracks.insert(0, track);
        }
        self.index = 0;
    }
}

fn generate_shuffle_seed() -> usize {
    let bytes = *Uuid::new_v4().as_bytes();
    bytes
        .iter()
        .take(std::mem::size_of::<usize>())
        .fold(0usize, |acc, byte| (acc << 8) ^ (*byte as usize))
}

fn sort_tracks_by_seed(tracks: &mut [Arc<Mutex<QueueTrack>>], seed: usize) {
    tracks.sort_by_key(|track| {
        let identity = track
            .lock()
            .ok()
            .map(|track| format!("{}:{}:{}", track.queue_id, track.database_id, track.path))
            .unwrap_or_default();
        stable_shuffle_key(&identity, seed)
    });
}

fn stable_shuffle_key(identity: &str, seed: usize) -> u64 {
    let mut hash = seed as u64 ^ 0x9e37_79b9_7f4a_7c15;
    for byte in identity.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
        hash ^= hash >> 32;
    }
    hash
}
