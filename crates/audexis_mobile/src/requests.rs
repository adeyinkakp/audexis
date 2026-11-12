use super::*;

impl MobileCore {
    pub(super) fn call_open(
        &mut self,
        request: &Value,
        op: &str,
        previous: &State,
    ) -> Result<Value, String> {
        match op {
            "updated_track" => {
                let id = string(request, "id")?;
                let current = self.track(id)?;
                let track: Track = serde_json::from_value(request["track"].clone())
                    .map_err(|_| "Invalid updated track")?;
                if track.id != id || track.deleted {
                    return Err("The updated song does not match this request".into());
                }
                if track.revision > current.revision {
                    if track.sha256 != current.sha256 {
                        self.remove_download(id)?;
                    }
                    self.state.tracks.insert(id.to_owned(), track);
                }
            }
            "updated_playlist" => {
                let playlist: Playlist = serde_json::from_value(request["playlist"].clone())
                    .map_err(|_| "Invalid updated playlist")?;
                if playlist.deleted || !safe_id(&playlist.id) {
                    return Err("Invalid updated playlist".into());
                }
                if self
                    .state
                    .playlists
                    .get(&playlist.id)
                    .is_none_or(|current| current.revision < playlist.revision)
                {
                    self.state.playlists.insert(playlist.id.clone(), playlist);
                }
            }
            "rewind" | "rewind_playlist_tracks" => {
                let year = request["year"]
                    .as_i64()
                    .and_then(|year| i32::try_from(year).ok())
                    .ok_or("Choose a valid year")?;
                let month = match request.get("month") {
                    None | Some(Value::Null) => None,
                    Some(value) => Some(
                        value
                            .as_u64()
                            .and_then(|month| u32::try_from(month).ok())
                            .ok_or("Choose a valid month")?,
                    ),
                };
                let library = self.library.as_ref().ok_or("Library is not open")?;
                return if op == "rewind_playlist_tracks" {
                    library.rewind_playlist_tracks(year, month)
                } else {
                    library.rewind(year, month)
                };
            }
            "songs" => {
                let search = request["search"].as_str().unwrap_or("");
                let offset = match request.get("offset") {
                    None => 0,
                    Some(value) => value
                        .as_u64()
                        .and_then(|offset| usize::try_from(offset).ok())
                        .ok_or("Invalid song offset")?,
                };
                return self
                    .library
                    .as_ref()
                    .ok_or("Library is not open")?
                    .songs(search, offset);
            }
            "snapshot" => return Ok(self.snapshot()),
            "identity" => {
                let id = string(request, "user_id")?;
                if self.state.user_id.as_deref().is_some_and(|old| old != id) {
                    return Err("The server account changed. Sign out and reconnect.".into());
                }
                self.state.user_id = Some(id.into());
            }
            "changes" => {
                let page: Changes = serde_json::from_value(request["page"].clone())
                    .map_err(|_| "Invalid cloud changes")?;
                if request["after"].as_i64() != Some(self.state.cursor) {
                    return Err("Stale sync page; refresh again".into());
                }
                let mut cursor = self.state.cursor;
                for change in &page.changes {
                    if change.cursor <= cursor {
                        return Err("Invalid cloud change order".into());
                    }
                    cursor = change.cursor;
                    if let ChangeValue::Track(t) = &change.value
                        && !safe_id(&t.id)
                    {
                        return Err("Invalid cloud track ID".into());
                    }
                }
                if page.next_cursor != cursor || (page.has_more && page.changes.is_empty()) {
                    return Err("Cloud sync cursor did not advance".into());
                }
                for change in page.changes {
                    let stale = match &change.value {
                        ChangeValue::Track(t) => self
                            .state
                            .tracks
                            .get(&t.id)
                            .is_some_and(|old| old.revision > t.revision),
                        ChangeValue::Playlist(p) => self
                            .state
                            .playlists
                            .get(&p.id)
                            .is_some_and(|old| old.revision > p.revision),
                    };
                    if stale {
                        continue;
                    }
                    match change.value {
                        ChangeValue::Track(t) if t.deleted => {
                            self.state.tracks.remove(&t.id);
                            self.remove_download(&t.id)?;
                        }
                        ChangeValue::Track(t) => {
                            if self
                                .state
                                .downloads
                                .get(&t.id)
                                .is_some_and(|d| d.source_hash != t.sha256)
                            {
                                self.remove_download(&t.id)?;
                            }
                            self.state.tracks.insert(t.id.clone(), t);
                        }
                        ChangeValue::Playlist(p) if p.deleted => {
                            self.state.playlists.remove(&p.id);
                        }
                        ChangeValue::Playlist(p) => {
                            self.state.playlists.insert(p.id.clone(), p);
                        }
                    }
                }
                self.state.cursor = cursor;
                let old_current = self.state.current;
                let mut current = None;
                let mut queue = Vec::new();
                for (index, id) in self.state.queue.iter().enumerate() {
                    if self.state.tracks.contains_key(id) {
                        if Some(index) == old_current {
                            current = Some(queue.len());
                        }
                        queue.push(id.clone());
                    }
                }
                self.state.queue = queue;
                self.state.current = current;
            }
            "play" => {
                let ids: Vec<String> =
                    serde_json::from_value(request["ids"].clone()).map_err(|_| "Invalid queue")?;
                let index = request["index"].as_u64().ok_or("Invalid queue position")? as usize;
                if index >= ids.len() {
                    return Err("Invalid queue position".into());
                }
                for id in &ids {
                    self.track(id)?;
                }
                self.state.queue = ids;
                self.state.current = Some(index);
            }
            "enqueue" => {
                let id = string(request, "id")?.to_owned();
                self.track(&id)?;
                if request["next"].as_bool() == Some(true) {
                    self.state.queue.insert(
                        self.state
                            .current
                            .map(|i| i + 1)
                            .unwrap_or(self.state.queue.len()),
                        id,
                    );
                } else {
                    self.state.queue.push(id);
                }
            }
            "select" => {
                let index = request["index"].as_u64().ok_or("Invalid queue position")? as usize;
                if index >= self.state.queue.len() {
                    return Err("Invalid queue position".into());
                }
                self.state.current = Some(index);
            }
            "advance" => {
                let previous = request["previous"].as_bool() == Some(true);
                self.state.current = match self.state.current {
                    Some(i) if previous => Some(i.saturating_sub(1)),
                    Some(i) if i + 1 < self.state.queue.len() => Some(i + 1),
                    _ => None,
                };
            }
            "remove_queue" => {
                let i = request["index"].as_u64().ok_or("Invalid queue position")? as usize;
                if i >= self.state.queue.len() {
                    return Err("Invalid queue position".into());
                }
                self.state.queue.remove(i);
                self.state.current = match self.state.current {
                    Some(c) if c == i => None,
                    Some(c) if c > i => Some(c - 1),
                    other => other,
                };
            }
            "clear_queue" => {
                self.state.queue.clear();
                self.state.current = None;
            }
            "media_plan" => return self.media_plan(request),
            "commit_download" => self.commit_download(request)?,
            "remove_download" => {
                self.remove_download(string(request, "id")?)?;
            }
            _ => return Err("Unknown mobile operation".into()),
        }
        self.library
            .as_ref()
            .ok_or("Library is not open")?
            .save(&self.state, previous)?;
        self.state = self.library.as_ref().ok_or("Library is not open")?.load()?;
        // Remove obsolete audio only after its database records have committed.
        for download in previous.downloads.values() {
            if !self
                .state
                .downloads
                .values()
                .any(|d| d.file == download.file)
            {
                let _ = fs::remove_file(self.root()?.join("audio").join(&download.file));
            }
        }
        Ok(self.snapshot())
    }
}
