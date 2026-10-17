use super::media::{MediaContent, MediaItem, MediaLayout, MediaTarget};
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
        let cards = self
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                let playback = playback.clone();
                let playlist = playlist.clone();
                MediaItem::new(MediaTarget::track(track.id, track.path.clone()))
                    .width(Size::px(166.0))
                    .child(
                        Button::new()
                            .flat()
                            .enabled(playback.available())
                            .child(MediaContent {
                                id: track.id,
                                path: track.path.clone(),
                                title: track.title.clone(),
                                subtitle: track.artist.clone(),
                                layout: MediaLayout::Card,
                                current: false,
                            })
                            .on_press(move |event: Event<PressEventData>| match event.data() {
                                PressEventData::Mouse(mouse) => {
                                    if EventsCombos::pressed(mouse.global_location).is_double() {
                                        playback.send(PlayerCmd::LoadAt {
                                            tracks: playlist.as_ref().clone(),
                                            index,
                                        })
                                    }
                                }
                                _ => (),
                            }),
                    )
                    .into_element()
            })
            .collect::<Vec<_>>();
        rect()
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
            )
    }
}
