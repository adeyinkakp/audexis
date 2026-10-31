use std::sync::{Arc, Mutex};

use uuid::Uuid;

mod navigation;

mod reconcile;

use shuffle::generate_shuffle_seed;

mod shuffle;
use crate::{
    AudioPlayerError, PlayerCmd,
    modes::{PlaybackModes, RepeatMode},
    queue_track::{QueueTrack, UnresolvedTrack},
};

pub struct LibraryReconcileResult {
    pub current_removed: bool,
    pub current_path_changed: bool,
    pub current_path: Option<String>,
}

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
        Ok(())
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

    pub fn queue_ids(&self) -> Vec<String> {
        self.tracks
            .iter()
            .filter_map(|track| track.lock().ok().map(|track| track.queue_id.clone()))
            .collect()
    }

    pub fn reorder(&mut self, queue_id: &str, target_id: &str) -> Result<(), String> {
        let ids = self.queue_ids();
        let from = ids
            .iter()
            .position(|id| id == queue_id)
            .ok_or("Queue entry is no longer available")?;
        let to = ids
            .iter()
            .position(|id| id == target_id)
            .ok_or("Target queue entry is no longer available")?;
        let current = self.current_queue_id();
        let track = self.tracks.remove(from);
        self.tracks.insert(to, track);
        if let Some(current) = current {
            self.index = self
                .queue_ids()
                .iter()
                .position(|id| id == &current)
                .unwrap() as i32;
        }
        if !self.shuffled {
            self.original_order = self.paths();
            self.original_queue_ids = self.queue_ids();
        }
        Ok(())
    }

    pub fn remove_queue_entries(&mut self, ids: &[String]) -> Result<bool, String> {
        let ids: std::collections::HashSet<_> = ids.iter().collect();

        let queue_ids = self
            .tracks
            .iter()
            .map(|track| {
                track
                    .lock()
                    .map(|track| track.queue_id.clone())
                    .map_err(|_| "Queue track is unavailable".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let current_index = self.index.max(0) as usize;
        let current_removed = queue_ids
            .get(current_index)
            .is_some_and(|id| ids.contains(id));
        let removed_before = queue_ids
            .iter()
            .take(current_index)
            .filter(|id| ids.contains(id))
            .count();
        let mut index = 0;
        self.tracks.retain(|_| {
            let keep = !ids.contains(&queue_ids[index]);
            index += 1;
            keep
        });
        let mut original_index = 0;
        self.original_queue_ids.retain(|id| {
            if ids.contains(id) {
                self.original_order.remove(original_index);
                false
            } else {
                original_index += 1;
                true
            }
        });
        self.index = current_index
            .saturating_sub(removed_before)
            .min(self.tracks.len()) as i32;
        if self.tracks.is_empty() {
            self.clear();
        }
        Ok(current_removed)
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
            if let Ok(track) = removed.lock()
                && let Some(original_index) = self
                    .original_queue_ids
                    .iter()
                    .position(|id| id == &track.queue_id)
            {
                self.original_queue_ids.remove(original_index);
                self.original_order.remove(original_index);
            }
            removed_current = position as i32 == self.index;
            if (position as i32) < self.index {
                self.index -= 1;
            }
        }
        for track in &self.tracks {
            if let Ok(mut track) = track.lock()
                && let Some(position) = track.playlist_ord
                && position > ord
            {
                track.playlist_ord = Some(position - 1);
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

    fn unload_all_tracks(&mut self) {
        for track in self.tracks.drain(..) {
            if let Ok(mut track) = track.lock() {
                track.unload();
            }
        }
    }
}
