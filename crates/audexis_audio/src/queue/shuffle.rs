use super::*;

impl Queue {
    pub(super) fn restore_original_order(&mut self, current_queue_id: Option<&str>) {
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
    pub(super) fn shuffle_tracks(&mut self, current_queue_id: Option<&str>) {
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
pub(super) fn generate_shuffle_seed() -> usize {
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
