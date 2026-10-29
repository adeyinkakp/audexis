use audexis_protocol::Track;
use serde::Serialize;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, SeekDirection};
#[cfg(not(target_os = "ios"))]
use souvlaki::{MediaPlayback, MediaPosition};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

#[cfg(target_os = "ios")]
mod ios;

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum RemoteAction {
    Play,
    Pause,
    Toggle,
    Stop,
    Next,
    Previous,
    Seek { position_ms: u64 },
    Skip { offset_ms: i64 },
}

#[derive(PartialEq)]
struct Metadata {
    title: String,
    artist: String,
    album: String,
    duration: u64,
    artwork: Option<PathBuf>,
}

#[derive(Default)]
pub(super) struct MobileMediaControls {
    controls: Option<MediaControls>,
    events: Option<Receiver<MediaControlEvent>>,
    track: Option<Track>,
    artwork: Option<PathBuf>,
    metadata: Option<Metadata>,
    index: Option<usize>,
    count: usize,
}

impl MobileMediaControls {
    pub(super) fn select(&mut self, track: Track) -> Result<(), String> {
        #[cfg(target_os = "ios")]
        let _pool = ios::Pool::new();
        if self.controls.is_none() {
            let mut controls = MediaControls::new(souvlaki::PlatformConfig {
                display_name: "Audexis",
                dbus_name: "audexis.mobile",
                hwnd: None,
            })
            .map_err(|error| error.to_string())?;
            let (sender, events) = mpsc::sync_channel(64);
            #[cfg(target_os = "ios")]
            let stop_sender = sender.clone();
            controls
                .attach(move |event| {
                    let _ = sender.try_send(event);
                })
                .map_err(|error| error.to_string())?;
            #[cfg(target_os = "ios")]
            ios::attach_stop(stop_sender);
            self.controls = Some(controls);
            self.events = Some(events);
        }
        if self.track.as_ref().map(|old| &old.id) != Some(&track.id) {
            self.artwork = None;
        }
        self.track = Some(track);
        Ok(())
    }

    pub(super) fn artwork(&mut self, path: PathBuf) {
        self.artwork = Some(path);
    }
    pub(super) fn queue(&mut self, index: Option<usize>, count: usize) {
        self.index = index;
        self.count = count;
    }

    pub(super) fn update(
        &mut self,
        playing: bool,
        position_ms: u64,
        duration_ms: u64,
    ) -> Result<(), String> {
        #[cfg(target_os = "ios")]
        let _pool = ios::Pool::new();
        let (Some(controls), Some(track)) = (&mut self.controls, &self.track) else {
            return Ok(());
        };
        let duration = if duration_ms > 0 {
            duration_ms
        } else {
            track.duration_ms.unwrap_or(0).max(0) as u64
        };
        let metadata = Metadata {
            title: track.title(),
            artist: track.artist(),
            album: track
                .metadata
                .tags
                .get("album")
                .and_then(|values| values.first())
                .cloned()
                .unwrap_or_default(),
            duration,
            artwork: self.artwork.clone(),
        };
        if self.metadata.as_ref() != Some(&metadata) {
            controls
                .set_metadata(MediaMetadata {
                    title: Some(&metadata.title),
                    artist: Some(&metadata.artist),
                    album: Some(&metadata.album),
                    duration: Some(Duration::from_millis(duration)),
                    cover_url: metadata.artwork.as_ref().and_then(|path| path.to_str()),
                })
                .map_err(|error| error.to_string())?;
            self.metadata = Some(metadata);
        }

        #[cfg(not(target_os = "ios"))]
        controls
            .set_playback(if playing {
                MediaPlayback::Playing {
                    progress: Some(MediaPosition(Duration::from_millis(position_ms))),
                }
            } else {
                MediaPlayback::Paused {
                    progress: Some(MediaPosition(Duration::from_millis(position_ms))),
                }
            })
            .map_err(|error| error.to_string())?;
        #[cfg(target_os = "ios")]
        ios::update(playing, position_ms, duration, self.index, self.count);
        Ok(())
    }

    pub(super) fn drain(&self) -> Vec<RemoteAction> {
        self.events
            .iter()
            .flat_map(|events| events.try_iter())
            .filter_map(|event| match event {
                MediaControlEvent::Play => Some(RemoteAction::Play),
                MediaControlEvent::Pause => Some(RemoteAction::Pause),
                MediaControlEvent::Toggle => Some(RemoteAction::Toggle),
                MediaControlEvent::Stop => Some(RemoteAction::Stop),
                MediaControlEvent::Next => Some(RemoteAction::Next),
                MediaControlEvent::Previous => Some(RemoteAction::Previous),
                MediaControlEvent::SetPosition(position) => Some(RemoteAction::Seek {
                    position_ms: position.0.as_millis().min(u128::from(u64::MAX)) as u64,
                }),
                MediaControlEvent::Seek(direction) => Some(RemoteAction::Skip {
                    offset_ms: if direction == SeekDirection::Forward {
                        10_000
                    } else {
                        -10_000
                    },
                }),
                MediaControlEvent::SeekBy(direction, duration) => {
                    let offset = duration.as_millis().min(i64::MAX as u128) as i64;
                    Some(RemoteAction::Skip {
                        offset_ms: if direction == SeekDirection::Forward {
                            offset
                        } else {
                            -offset
                        },
                    })
                }
                _ => None,
            })
            .collect()
    }

    pub(super) fn clear(&mut self) {
        #[cfg(target_os = "ios")]
        let _pool = ios::Pool::new();
        if let Some(mut controls) = self.controls.take() {
            let _ = controls.set_metadata(MediaMetadata::default());
            #[cfg(not(target_os = "ios"))]
            let _ = controls.set_playback(MediaPlayback::Stopped);
            drop(controls);
        }
        #[cfg(target_os = "ios")]
        ios::clear();
        self.events = None;
        self.track = None;
        self.artwork = None;
        self.metadata = None;
    }
}

impl Drop for MobileMediaControls {
    fn drop(&mut self) {
        if self.controls.is_some() {
            self.clear();
        }
    }
}
