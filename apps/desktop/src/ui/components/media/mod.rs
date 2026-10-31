use crate::ui::{hooks::use_playback, theme::use_audexis_colors};
use audexis_audio::{PlayerCmd, queue_track::UnresolvedTrack};
use freya::prelude::*;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct MediaTrack {
    pub id: i64,
    pub path: String,
}
impl MediaTrack {
    fn unresolved(&self) -> UnresolvedTrack {
        UnresolvedTrack {
            id: self.id,
            path: self.path.clone(),
            occurrence: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectedTrack {
    pub track: MediaTrack,
    pub queue_id: Option<String>,
    pub playlist: Option<PlaylistOccurrence>,
}
impl SelectedTrack {
    pub fn key(&self) -> String {
        if let Some(id) = &self.queue_id {
            format!("queue:{id}")
        } else if let Some(p) = self.playlist {
            format!("playlist:{}:{}:{}", p.playlist, p.ord, p.file)
        } else {
            format!("file:{}:{}", self.track.id, self.track.path)
        }
    }
    fn unresolved(&self) -> UnresolvedTrack {
        UnresolvedTrack {
            id: self.track.id,
            path: self.track.path.clone(),
            occurrence: self.playlist.map(|p| p.ord),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MediaTarget {
    Selection {
        tracks: Arc<Vec<SelectedTrack>>,
    },
    Track {
        track: MediaTrack,
        queue_id: Option<String>,
        query: Option<audexis_core::database::songs::SongQuery>,
    },

    Album {
        tracks: Arc<Vec<MediaTrack>>,
    },
}
impl MediaTarget {
    pub fn track(id: i64, path: String) -> Self {
        Self::Track {
            track: MediaTrack { id, path },
            queue_id: None,
            query: None,
        }
    }
    pub fn queue_track(id: i64, path: String, queue_id: String) -> Self {
        Self::Track {
            track: MediaTrack { id, path },
            queue_id: Some(queue_id),
            query: None,
        }
    }
    pub fn library_track(
        id: i64,
        path: String,
        query: audexis_core::database::songs::SongQuery,
    ) -> Self {
        Self::Track {
            track: MediaTrack { id, path },
            queue_id: None,
            query: Some(query),
        }
    }
    pub fn play(&self, playback: &crate::ui::hooks::Playback) {
        if let Self::Track {
            track,
            query: Some(query),
            ..
        } = self
        {
            playback.play_library(query.clone(), track.id);
        } else if let Some(command) = self.play_command() {
            playback.send(command);
        }
    }
    pub fn play_command(&self) -> Option<PlayerCmd> {
        match self {
            Self::Selection { tracks }
                if !tracks.is_empty() && tracks.iter().all(|t| !t.track.path.is_empty()) =>
            {
                Some(PlayerCmd::Load {
                    tracks: tracks.iter().map(SelectedTrack::unresolved).collect(),
                })
            }
            Self::Track {
                queue_id: Some(queue_id),
                ..
            } => Some(PlayerCmd::PlayQueueEntry {
                queue_id: queue_id.clone(),
            }),
            Self::Track { track, .. } if !track.path.is_empty() => Some(PlayerCmd::Load {
                tracks: vec![track.unresolved()],
            }),
            Self::Album { tracks }
                if !tracks.is_empty() && tracks.iter().all(|t| !t.path.is_empty()) =>
            {
                Some(PlayerCmd::Load {
                    tracks: tracks.iter().map(MediaTrack::unresolved).collect(),
                })
            }
            _ => None,
        }
    }
    pub(crate) fn enqueue_command(&self, next: bool) -> Option<PlayerCmd> {
        let tracks = match self {
            Self::Selection { tracks } => tracks.iter().map(SelectedTrack::unresolved).collect(),
            Self::Track { track, .. } => vec![track.unresolved()],
            Self::Album { tracks } => tracks.iter().map(MediaTrack::unresolved).collect(),
        };
        if tracks.is_empty() || tracks.iter().any(|track| track.path.is_empty()) {
            return None;
        }
        Some(PlayerCmd::Enqueue { tracks, next })
    }
    pub(crate) fn paths(&self) -> String {
        match self {
            Self::Selection { tracks } => tracks
                .iter()
                .map(|t| t.track.path.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            Self::Track { track, .. } => track.path.clone(),
            Self::Album { tracks } => tracks
                .iter()
                .map(|t| t.path.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaylistOccurrence {
    pub playlist: i64,
    pub ord: i64,
    pub file: i64,
}

#[derive(PartialEq)]
pub struct MediaItem {
    target: MediaTarget,
    children: Vec<Element>,
    width: Size,
    playlist: Option<PlaylistOccurrence>,
}
impl MediaItem {
    pub fn new(target: MediaTarget) -> Self {
        Self {
            target,
            children: Vec::new(),
            width: Size::fill(),
            playlist: None,
        }
    }
    pub fn playlist(mut self, playlist: Option<PlaylistOccurrence>) -> Self {
        self.playlist = playlist;
        self
    }
    pub fn width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }
}
impl ChildrenExt for MediaItem {
    fn get_children(&mut self) -> &mut Vec<Element> {
        &mut self.children
    }
}
impl Component for MediaItem {
    fn render(&self) -> impl IntoElement {
        let target = self.target.clone();
        let services = use_consume::<Arc<crate::services::AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let playback = use_playback();
        let router = freya::router::RouterContext::try_get();
        let playlist = self.playlist;
        let busy = use_state(|| false);
        rect()
            .width(self.width.clone())
            .children(self.children.clone())
            .on_secondary_down(move |event: Event<PressEventData>| {
                event.stop_propagation();
                event.prevent_default();
                crate::ui::menus::context::open(
                    &event,
                    target.clone(),
                    playback.clone(),
                    services.clone(),
                    runtime.clone(),
                    router.clone(),
                    playlist,
                    busy,
                );
            })
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum MediaLayout {
    Card,
    Row { artwork_size: f32, title_size: f32 },
}

#[derive(PartialEq)]
pub struct MediaContent {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub subtitle: String,
    pub layout: MediaLayout,
    pub current: bool,
}
impl Component for MediaContent {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let (size, title_size) = match self.layout {
            MediaLayout::Card => (150.0, 14.0),
            MediaLayout::Row {
                artwork_size,
                title_size,
            } => (artwork_size, title_size),
        };
        let card = self.layout == MediaLayout::Card;
        rect()
            .width(if card { Size::px(166.0) } else { Size::fill() })
            .direction(if card {
                Direction::Vertical
            } else {
                Direction::Horizontal
            })
            .content(Content::Flex)
            .cross_align(if card {
                Alignment::Start
            } else {
                Alignment::Center
            })
            .spacing(8.0)
            .padding(if card { 8.0 } else { 0.0 })
            .child(super::Artwork {
                file_id: self.id,
                path: self.path.clone(),
                size,
            })
            .child(
                rect()
                    .width(if card { Size::fill() } else { Size::flex(1.0) })
                    .spacing(2.0)
                    .child(
                        label()
                            .text(crate::ui::display_text(self.title.clone()))
                            .width(Size::fill())
                            .max_lines(if card { 2 } else { 1 })
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(title_size)
                            .font_weight(FontWeight::BOLD)
                            .color(if self.current {
                                colors.primary
                            } else {
                                colors.foreground
                            }),
                    )
                    .child(
                        label()
                            .text(crate::ui::display_text(self.subtitle.clone()))
                            .width(Size::fill())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(title_size - 2.0)
                            .color(colors.muted_foreground),
                    ),
            )
    }
}

#[derive(PartialEq)]
pub struct AlbumItem {
    pub title: String,
    pub artist: String,
    pub tracks: Arc<Vec<MediaTrack>>,
    pub layout: MediaLayout,
}
impl Component for AlbumItem {
    fn render(&self) -> impl IntoElement {
        let target = MediaTarget::Album {
            tracks: self.tracks.clone(),
        };
        let playback = use_playback();
        let first = self.tracks.first();
        MediaItem::new(target.clone())
            .width(if self.layout == MediaLayout::Card {
                Size::px(166.0)
            } else {
                Size::fill()
            })
            .child(
                Button::new()
                    .flat()
                    .expanded()
                    .enabled(playback.available() && target.play_command().is_some())
                    .child(MediaContent {
                        id: first.map_or(0, |t| t.id),
                        path: first.map(|t| t.path.clone()).unwrap_or_default(),
                        title: self.title.clone(),
                        subtitle: self.artist.clone(),
                        layout: self.layout,
                        current: false,
                    })
                    .on_press(move |_| {
                        if let Some(command) = target.play_command() {
                            playback.send(command);
                        }
                    }),
            )
    }
}
