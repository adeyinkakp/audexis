mod library;
mod media;
use media::audio_format;
#[cfg(not(target_os = "ios"))]
mod media_controls;
#[cfg(target_os = "ios")]
#[path = "media_controls/ios.rs"]
mod media_controls;
mod persistence;
mod playback;
mod requests;
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
    quality: Option<String>,
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
