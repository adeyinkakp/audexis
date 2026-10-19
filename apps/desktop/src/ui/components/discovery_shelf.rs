use super::media::{MediaContent, MediaLayout, MediaTarget, MediaTrack, SelectedTrack};
use super::track_selection::{SelectableTrack, TrackSelectionScope};
use crate::ui::{hooks::use_playback, theme::use_audexis_colors};
use audexis_audio::{PlayerCmd, queue_track::UnresolvedTrack};
use audexis_core::database::discovery::DiscoveryTrack;
use freya::prelude::*;

#[derive(PartialEq)]
pub struct DiscoveryShelf {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub empty: &'static str,
    pub tracks: Vec<DiscoveryTrack>,
}
impl Component for DiscoveryShelf {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let playback = use_playback();
        let error = playback.error;
        let playlist = std::sync::Arc::new(
            self.tracks
                .iter()
                .map(|track| UnresolvedTrack {
                    id: track.id,
                    path: track.path.clone(),
                    occurrence: None,
                })
                .collect::<Vec<_>>(),
        );
        let items = std::sync::Arc::new(
            self.tracks
                .iter()
                .map(|track| SelectedTrack {
                    track: MediaTrack {
                        id: track.id,
                        path: track.path.clone(),
                    },
                    queue_id: None,
                    playlist: None,
                })
                .collect::<Vec<_>>(),
        );
        let cards = self
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                let playback = playback.clone();
                let playlist = playlist.clone();
                SelectableTrack::new(
                    items[index].clone(),
                    MediaTarget::track(track.id, track.path.clone()),
                    EventHandler::new(move |()| {
                        playback.send(PlayerCmd::LoadAt {
                            tracks: playlist.as_ref().clone(),
                            index,
                        })
                    }),
                )
                .width(Size::px(166.))
                .child(MediaContent {
                    id: track.id,
                    path: track.path.clone(),
                    title: track.title.clone(),
                    subtitle: track.artist.clone(),
                    layout: MediaLayout::Card,
                    current: false,
                })
                .into_element()
            })
            .collect::<Vec<_>>();
        let body = rect()
            .width(Size::fill())
            .spacing(12.0)
            .child(
                label()
                    .text(self.title)
                    .font_size(23.0)
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                label()
                    .text(self.subtitle)
                    .font_size(13.0)
                    .color(colors.muted_foreground),
            )
            .maybe_child(self.tracks.is_empty().then(|| {
                rect()
                    .width(Size::fill())
                    .padding(20.0)
                    .corner_radius(12.0)
                    .background(colors.surface_subtle)
                    .child(label().text(self.empty).color(colors.muted_foreground))
            }))
            .maybe_child((!self.tracks.is_empty()).then(|| {
                rect().width(Size::fill()).height(Size::px(265.0)).child(
                    super::SmoothScroll::new()
                        .horizontal()
                        .spacing(12.0)
                        .children(cards),
                )
            }))
            .maybe_child(
                error
                    .read()
                    .clone()
                    .map(|message| label().text(message).color(colors.destructive)),
            );
        TrackSelectionScope {
            items,
            child: body.into_element(),
            fill: false,
            on_select_all: None,
        }
    }
}
