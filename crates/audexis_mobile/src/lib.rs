mod library;
mod media_controls;
mod persistence;
mod playback;
mod storage;

mod ffi;
pub use ffi::*;

use audexis_protocol::{ChangeValue, Changes, MediaUrl, Playlist, Track};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::{CStr, CString, c_char},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Default)]
struct State {
    generation: i64,
    cursor: i64,
    user_id: Option<String>,
    tracks: BTreeMap<String, Track>,
    playlists: BTreeMap<String, Playlist>,
    queue: Vec<String>,
    current: Option<usize>,
    downloads: BTreeMap<String, Download>,
}
#[derive(Clone)]
struct Download {
    file: String,
    source_hash: String,
    sha256: String,
    size: i64,
}

#[derive(Default)]
pub struct MobileCore {
    playback: playback::MobilePlayback,
    library: Option<library::CoreLibrary>,
    root: Option<PathBuf>,
    state: State,
}

impl MobileCore {
    pub fn call(&mut self, request: Value) -> Result<Value, String> {
        let op = request["op"].as_str().ok_or("Missing operation")?;
        if op == "open" {
            self.playback.reset();
            let root = PathBuf::from(string(&request, "directory")?);
            fs::create_dir_all(root.join("audio")).map_err(|e| e.to_string())?;
            let library = library::CoreLibrary::open(&root)?;
            let state = library.open_state()?;
            self.library = Some(library);
            self.root = Some(root);
            self.state = state;
            return Ok(self.snapshot());
        }
        self.root()?;
        if op.starts_with("player_") {
            return self.player_call(&request, op);
        }
        self.state = self.library.as_ref().ok_or("Library is not open")?.load()?;
        let previous = self.state.clone();
        let result = self.call_open(&request, op, &previous);
        if result.is_err() {
            self.state = previous;
        }
        result
    }

    fn call_open(&mut self, request: &Value, op: &str, previous: &State) -> Result<Value, String> {
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
            "rewind" => {
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
                return self
                    .library
                    .as_ref()
                    .ok_or("Library is not open")?
                    .rewind(year, month);
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
                let id = string(&request, "user_id")?;
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
                // Retain the current entry's position even when earlier queue entries disappear.
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
                let id = string(&request, "id")?.to_owned();
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
            "media_plan" => {
                let track = self.track(string(&request, "id")?)?;
                let automatic = if native_format(&audio_format(track)) {
                    "original"
                } else {
                    "aac_256"
                };
                let quality = match request.get("quality") {
                    None | Some(Value::Null) => automatic,
                    Some(Value::String(quality)) if quality == "automatic" => automatic,
                    Some(Value::String(quality))
                        if matches!(quality.as_str(), "original" | "aac_128" | "aac_256") =>
                    {
                        quality.as_str()
                    }
                    _ => return Err("Unknown audio quality".into()),
                };
                return Ok(
                    json!({"path":format!("/api/tracks/{}/audio?quality={quality}", track.id),"quality":quality}),
                );
            }
            "commit_download" => {
                let id = string(&request, "id")?;
                let track = self.track(id)?.clone();
                let source_hash = string(&request, "source_hash")?;
                if track.sha256 != source_hash {
                    return Err("This track changed while downloading; try again".into());
                }
                let media: MediaUrl = serde_json::from_value(request["media"].clone())
                    .map_err(|_| "Invalid download information")?;
                let path = PathBuf::from(string(&request, "temporary")?);
                if media.size <= 0
                    || media.sha256.len() != 64
                    || !media.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    return Err("Invalid download information".into());
                }
                let mut file = fs::File::open(&path).map_err(|e| e.to_string())?;
                if file.metadata().map_err(|e| e.to_string())?.len() != media.size as u64 {
                    return Err("Downloaded audio has the wrong size".into());
                }
                let mut hash = Sha256::new();
                let mut buffer = [0u8; 64 * 1024];
                loop {
                    let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
                    if count == 0 {
                        break;
                    }
                    hash.update(&buffer[..count]);
                }
                if format!("{:x}", hash.finalize()) != media.sha256 {
                    return Err("Downloaded audio failed verification".into());
                }
                let extension = if media.quality == "original" {
                    extension(&audio_format(&track))
                } else {
                    "m4a"
                };
                let name = format!("{}.{extension}", media.sha256);
                let destination = self.root()?.join("audio").join(&name);
                if destination != path {
                    fs::copy(&path, &destination).map_err(|e| e.to_string())?;
                }
                self.state.downloads.insert(
                    id.into(),
                    Download {
                        file: name,
                        source_hash: track.sha256,
                        sha256: media.sha256,
                        size: media.size,
                    },
                );
            }
            "remove_download" => {
                self.remove_download(string(&request, "id")?)?;
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
    fn remove_download(&mut self, id: &str) -> Result<(), String> {
        self.state.downloads.remove(id);
        Ok(())
    }
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key].as_str().ok_or_else(|| format!("Missing {key}"))
}
fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 512
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
}
fn audio_format(track: &Track) -> String {
    Path::new(&track.file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or(&track.format)
        .to_ascii_lowercase()
}
fn native_format(format: &str) -> bool {
    matches!(
        format.to_ascii_lowercase().as_str(),
        "mp3" | "m4a" | "mp4" | "aac" | "wav" | "aiff" | "aif" | "alac"
    )
}
fn extension(format: &str) -> &'static str {
    match format {
        "mp3" => "mp3",
        "aac" => "aac",
        "wav" => "wav",
        "aiff" | "aif" => "aiff",
        "flac" => "flac",
        "ogg" | "oga" => "ogg",
        "opus" => "opus",
        "mp1" => "mp1",
        "mp2" => "mp2",
        "spx" => "spx",
        "m4b" => "m4b",
        "mov" => "mov",
        "m4v" => "m4v",
        "ogv" => "ogv",
        _ => "m4a",
    }
}
