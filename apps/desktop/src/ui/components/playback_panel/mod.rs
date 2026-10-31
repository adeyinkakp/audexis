use super::media::MediaTarget;
use super::player_controls::{ControlSize, icon_button};
use super::{Artwork, VisibilityViewport};

mod lines;
mod lyrics;
mod queue;

use lyrics::LyricsPanel;
use queue::QueuePanel;

use crate::{
    services::AppServices,
    ui::{
        hooks::{PlaybackPanelView, use_library, use_playback},
        theme::use_audexis_colors,
    },
};
use audexis_audio::{PlayerCmd, QueueInfo};
use freya::{icons::lucide, prelude::*};
use std::sync::Arc;

#[derive(PartialEq)]
pub struct PlaybackPanel;
#[derive(PartialEq)]
pub(crate) struct ExpandedPlaybackPanel {
    pub stacked: bool,
}
impl Component for ExpandedPlaybackPanel {
    fn render(&self) -> impl IntoElement {
        rect()
            .width(if self.stacked {
                Size::fill()
            } else {
                Size::flex(1.)
            })
            .height(if self.stacked {
                Size::flex(1.)
            } else {
                Size::fill()
            })
            .child(PanelContent)
    }
}
impl Component for PlaybackPanel {
    fn render(&self) -> impl IntoElement {
        let playback = use_playback();
        let mut panel = playback.panel;
        let view = *panel.read();
        let colors = use_audexis_colors();
        let Some(view) = view else {
            return rect().into_element();
        };
        rect()
            .width(Size::px(320.0))
            .height(Size::fill())
            .content(Content::Flex)
            .background(colors.popover)
            .border(Border::new().width(1.0).fill(colors.border))
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(48.0))
                    .padding(8.0)
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .spacing(4.0)
                    .child(
                        Button::new()
                            .compact()
                            .flat()
                            .child(label().text("Queue").color(
                                if view == PlaybackPanelView::Queue {
                                    colors.primary
                                } else {
                                    colors.muted_foreground
                                },
                            ))
                            .on_press(move |_| panel.set(Some(PlaybackPanelView::Queue))),
                    )
                    .child(
                        Button::new()
                            .compact()
                            .flat()
                            .child(label().text("Lyrics").color(
                                if view == PlaybackPanelView::Lyrics {
                                    colors.primary
                                } else {
                                    colors.muted_foreground
                                },
                            ))
                            .on_press(move |_| panel.set(Some(PlaybackPanelView::Lyrics))),
                    )
                    .child(rect().width(Size::flex(1.0)))
                    .child(icon_button(
                        "Close panel",
                        SvgViewer::new(("x", lucide::x())),
                        true,
                        ControlSize::Panel,
                        move |_| panel.set(None),
                    )),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.0))
                    .child(PanelContent),
            )
            .maybe_child(
                playback
                    .error
                    .read()
                    .clone()
                    .map(|e| label().text(e).font_size(11.0).color(colors.destructive)),
            )
            .into_element()
    }
}
#[derive(PartialEq)]
struct PanelContent;
impl Component for PanelContent {
    fn render(&self) -> impl IntoElement {
        match *use_playback().panel.read() {
            Some(PlaybackPanelView::Queue) => QueuePanel.into_element(),
            Some(PlaybackPanelView::Lyrics) => LyricsPanel.into_element(),
            None => rect().into_element(),
        }
    }
}
