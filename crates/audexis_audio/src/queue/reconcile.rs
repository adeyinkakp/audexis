use super::*;

impl Queue {
    pub fn reconcile_library_files(
        &mut self,
        path_updates: &std::collections::HashMap<i64, String>,
        missing_ids: &std::collections::HashSet<i64>,
    ) -> LibraryReconcileResult {
        let current_queue_id = self.current_queue_id();
        let current_index = self.index.max(0) as usize;
        let mut current_path_changed = false;

        for track in &self.tracks {
            if let Ok(mut track) = track.lock()
                && let Some(path) = path_updates.get(&track.database_id)
            {
                if current_queue_id.as_deref() == Some(track.queue_id.as_str())
                    && track.path != *path
                {
                    current_path_changed = true;
                }
                track.path.clone_from(path);
                if let Some(original_index) = self
                    .original_queue_ids
                    .iter()
                    .position(|queue_id| queue_id == &track.queue_id)
                {
                    self.original_order[original_index].clone_from(path);
                }
            }
        }

        let removed_before_current = self
            .tracks
            .iter()
            .take(current_index)
            .filter(|track| {
                track
                    .lock()
                    .ok()
                    .is_some_and(|track| missing_ids.contains(&track.database_id))
            })
            .count();
        let current_removed = self
            .tracks
            .get(current_index)
            .and_then(|track| track.lock().ok())
            .is_some_and(|track| missing_ids.contains(&track.database_id));

        let removed_queue_ids = self
            .tracks
            .iter()
            .filter_map(|track| {
                track.lock().ok().and_then(|track| {
                    missing_ids
                        .contains(&track.database_id)
                        .then(|| track.queue_id.clone())
                })
            })
            .collect::<std::collections::HashSet<_>>();
        self.tracks.retain(|track| {
            track
                .lock()
                .map(|track| !missing_ids.contains(&track.database_id))
                .unwrap_or(false)
        });
        let mut original_index = 0;
        self.original_queue_ids.retain(|queue_id| {
            let keep = !removed_queue_ids.contains(queue_id);
            if !keep {
                self.original_order.remove(original_index);
            } else {
                original_index += 1;
            }
            keep
        });

        if self.tracks.is_empty() {
            self.index = 0;
        } else if current_removed {
            self.index = current_index
                .saturating_sub(removed_before_current)
                .min(self.tracks.len() - 1) as i32;
        } else if let Some(queue_id) = current_queue_id {
            self.index = self
                .tracks
                .iter()
                .position(|track| {
                    track
                        .lock()
                        .ok()
                        .is_some_and(|track| track.queue_id == queue_id)
                })
                .unwrap_or(0) as i32;
        }

        LibraryReconcileResult {
            current_removed,
            current_path_changed,
            current_path: self.current_path(),
        }
    }
}
