use super::{use_playback, use_songs};
use std::path::Path;

pub(crate) struct TrackMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub available: bool,
    cloud: bool,
}
impl TrackMetadata {
    pub fn subtitle(&self) -> String {
        if self.cloud {
            return self.artist.clone();
        }
        [&self.artist, &self.album]
            .into_iter()
            .filter(|value| !value.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

pub(crate) fn use_track_metadata(id: i64, path: Option<&str>) -> TrackMetadata {
    let songs = use_songs(&[id]);
    let song = songs.data.as_ref().and_then(|songs| songs.first());
    let mut metadata = TrackMetadata {
        title: song.map(|song| song.title.clone()).unwrap_or_else(|| {
            path.and_then(|path| Path::new(path).file_stem())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Nothing is playing".into())
        }),
        artist: song
            .map(|song| song.artist.clone())
            .unwrap_or_else(|| if path.is_some() { "Local audio" } else { "" }.into()),
        album: song.map(|song| song.album.clone()).unwrap_or_default(),
        available: song.is_some(),
        cloud: false,
    };
    let cloud = use_playback().cloud_track;
    if let Some((cloud_path, track)) = &*cloud.read() {
        if path == Some(cloud_path.as_str()) {
            metadata.title = track.title();
            metadata.artist = track.artist();
            metadata.album = track
                .metadata
                .tags
                .get("album")
                .and_then(|values| values.first())
                .cloned()
                .unwrap_or_default();
            metadata.available = true;
            metadata.cloud = true;
        }
    }
    metadata
}
