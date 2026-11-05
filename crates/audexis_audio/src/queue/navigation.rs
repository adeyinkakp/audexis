use super::*;

impl Queue {
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
    pub fn next_after_eof_index(&self) -> Option<usize> {
        self.next_after_index(self.index as usize)
    }
    pub fn next_after_index(&self, index: usize) -> Option<usize> {
        if index >= self.tracks.len() {
            return None;
        }
        match self.repeat_mode {
            RepeatMode::Track => Some(index),
            RepeatMode::Queue => Some((index + 1) % self.tracks.len()),
            RepeatMode::Off => (index + 1 < self.tracks.len()).then_some(index + 1),
        }
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
}
