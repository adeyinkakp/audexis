use super::{MobileCore, audio_format, library::CoreLibrary, string};
use audexis_audio::{
    AudioEvent, AudioPlayer, PlaybackState, PlayerCmd, PlayerHandle,
    queue_track::UnresolvedTrack,
    streaming::{StreamOptions, StreamRegistration},
};
use audexis_protocol::{MediaUrl, Track};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, watch};

#[derive(Default)]
pub(super) struct MobilePlayback {
    player: Option<Arc<Mutex<AudioPlayer>>>,
    handle: Option<PlayerHandle>,
    state: Option<watch::Receiver<PlaybackState>>,
    events: Option<broadcast::Receiver<AudioEvent>>,
    source: Option<Arc<StreamRegistration>>,
    controls: super::media_controls::MobileMediaControls,
    controls_error: Option<String>,
    revision: u64,
    track_id: Option<String>,
    started: bool,
    finished: bool,
    error: Option<String>,
}

impl MobilePlayback {
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    fn stop(&mut self, revision: u64) {
        self.player = None;
        self.handle = None;
        self.state = None;
        self.events = None;
        self.source = None;
        self.revision = revision;
        self.track_id = None;
        self.started = false;
        self.finished = false;
        self.error = None;
    }

    fn send(&self, command: PlayerCmd) -> Result<(), String> {
        self.handle
            .as_ref()
            .ok_or("No song is loaded")?
            .send(command)
    }

    fn load(
        &mut self,
        library: &CoreLibrary,
        track: &Track,
        request: &Value,
    ) -> Result<(), String> {
        let revision = request["playback_revision"]
            .as_u64()
            .ok_or("Missing playback revision")?;
        self.stop(revision);
        let (path, source) = if let Some(path) = request["path"].as_str() {
            (path.to_owned(), None)
        } else {
            let media: MediaUrl = serde_json::from_value(request["media"].clone())
                .map_err(|_| "Missing playback source")?;
            if media.size <= 0
                || !(media.url.starts_with("https://") || media.url.starts_with("http://"))
            {
                return Err("Invalid audio source".into());
            }
            let extension = if media.quality == "original" {
                audio_format(track)
            } else {
                "m4a".into()
            };
            let url = media.url;
            let source = StreamRegistration::with_options(
                media.size as u64,
                track.duration_ms.unwrap_or(0).max(0) as u64,
                extension,
                StreamOptions {
                    prefetch: false,
                    ..Default::default()
                },
                move || Ok(url.clone()),
            );
            (source.path().to_owned(), Some(source))
        };
        let file_id: i64 = library.runtime.block_on(
            sqlx::query_scalar("SELECT file_id FROM file_cloud_links WHERE connection_id='mobile' AND cloud_track_id=?")
                .bind(&track.id).fetch_one(&library.database.pool)
        ).map_err(|error| error.to_string())?;
        let player = AudioPlayer::new(
            None,
            library.database.pool.clone(),
            library.runtime.handle().clone(),
            Default::default(),
        )
        .map_err(|error| error.to_string())?;
        let handle = player
            .lock()
            .map_err(|_| "Audio player is unavailable")?
            .handle();
        self.state = Some(handle.subscribe());
        self.events = Some(handle.subscribe_events());
        self.source = source;
        self.track_id = Some(track.id.clone());
        self.player = Some(player);
        self.handle = Some(handle);
        self.send(PlayerCmd::LoadAt {
            tracks: vec![UnresolvedTrack {
                id: file_id,
                path,
                occurrence: None,
            }],
            index: 0,
        })
    }

    fn snapshot(&mut self) -> Value {
        if let Some(events) = &mut self.events {
            loop {
                match events.try_recv() {
                    Ok(AudioEvent::TrackStarted { .. }) => self.started = true,
                    Ok(AudioEvent::QueueFinished) => self.finished = true,
                    Ok(AudioEvent::Error(error)) => self.error = Some(error),
                    Ok(_) | Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        }
        let state = self
            .state
            .as_ref()
            .map(|state| state.borrow().clone())
            .unwrap_or_default();
        let error = self.error.as_ref().or(state.error.as_ref());
        let playing = self.started && !state.paused && !self.finished && error.is_none();
        self.controls_error = self
            .controls
            .update(playing, state.current_position, state.duration)
            .err();
        json!({
            "revision": self.revision,
            "track_id": self.track_id,
            "position_ms": state.current_position,
            "duration_ms": state.duration,
            "is_playing": playing,
            "is_loading": self.track_id.is_some() && !self.started && !self.finished && error.is_none(),
            "finished": self.finished && error.is_none(),
            "error": error,
            "controls_error": self.controls_error,
            "remote_actions": [],
        })
    }
}

impl MobileCore {
    pub(super) fn player_call(&mut self, request: &Value, op: &str) -> Result<Value, String> {
        self.playback
            .controls
            .queue(self.state.current, self.state.queue.len());
        if op == "player_state" {
            let mut snapshot = self.playback.snapshot();
            snapshot["remote_actions"] = serde_json::to_value(self.playback.controls.drain())
                .map_err(|error| error.to_string())?;
            return Ok(snapshot);
        }
        let revision = request["playback_revision"]
            .as_u64()
            .ok_or("Missing playback revision")?;
        if revision < self.playback.revision {
            return Ok(self.playback.snapshot());
        }
        match op {
            "player_stop" => {
                self.playback.stop(revision);
                self.playback.controls.clear();
            }
            "player_metadata" => {
                let id = string(request, "id")?;
                if self
                    .state
                    .current
                    .and_then(|index| self.state.queue.get(index))
                    .map(String::as_str)
                    != Some(id)
                {
                    return Ok(self.playback.snapshot());
                }
                let track = self.track(id)?.clone();
                self.playback.controls.select(track)?;
                self.playback.track_id = Some(id.into());
                self.playback.revision = revision;
            }
            "player_artwork" if revision == self.playback.revision => {
                if request["id"].as_str() == self.playback.track_id.as_deref() {
                    let path = std::fs::canonicalize(string(request, "path")?)
                        .map_err(|error| error.to_string())?;
                    let artwork_root = std::fs::canonicalize(self.root()?.join("artwork"))
                        .map_err(|error| error.to_string())?;
                    if !path.starts_with(artwork_root) || !path.is_file() {
                        return Err("Artwork must come from the library cache".into());
                    }
                    self.playback.controls.artwork(path);
                }
            }
            "player_queue" => {}
            "player_load" => {
                let id = string(request, "id")?;
                if self
                    .state
                    .current
                    .and_then(|index| self.state.queue.get(index))
                    .map(String::as_str)
                    != Some(id)
                {
                    return Err("This song is no longer selected".into());
                }
                let track = self.track(id)?.clone();
                if let Some(path) = request["path"].as_str() {
                    let download = self
                        .state
                        .downloads
                        .get(id)
                        .ok_or("This song is not downloaded")?;
                    if self.root()?.join("audio").join(&download.file) != std::path::Path::new(path)
                    {
                        return Err("The playback file does not match this download".into());
                    }
                }
                self.playback.load(
                    self.library.as_ref().ok_or("Library is not open")?,
                    &track,
                    request,
                )?;
            }
            _ if revision != self.playback.revision => return Ok(self.playback.snapshot()),
            "player_pause" => self.playback.send(PlayerCmd::Pause)?,
            "player_resume" => self.playback.send(PlayerCmd::Play { resuming: true })?,
            "player_seek" => {
                let milliseconds = request["position_ms"]
                    .as_u64()
                    .ok_or("Invalid playback position")?;
                self.playback
                    .send(PlayerCmd::SeekMilliseconds { milliseconds })?;
            }
            "player_rebuild" => {
                if let Some(player) = &self.playback.player {
                    player
                        .lock()
                        .map_err(|_| "Audio player is unavailable")?
                        .rebuild_active_stream(player.clone())
                        .map_err(|error| error.to_string())?;
                }
            }
            _ => return Err("Unknown player operation".into()),
        }
        Ok(self.playback.snapshot())
    }
}
