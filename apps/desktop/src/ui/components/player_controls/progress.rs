use super::super::compact_seek::CompactSeek;
use crate::ui::{format_time, hooks::use_playback, theme::use_audexis_colors};
use freya::prelude::*;

#[derive(PartialEq)]
pub(crate) struct InlinePlaybackProgress {
    pub position: u64,
    pub duration: u64,
    pub enabled: bool,
}
impl Component for InlinePlaybackProgress {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        rect()
            .width(Size::fill())
            .height(Size::px(16.))
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(8.)
            .color(colors.muted_foreground)
            .child(
                label()
                    .text(format_time(self.position))
                    .font_size(11.)
                    .max_lines(1),
            )
            .child(
                rect()
                    .width(Size::flex(1.))
                    .height(Size::px(16.))
                    .child(CompactSeek {
                        position: self.position,
                        duration: self.duration,
                        enabled: self.enabled,
                    }),
            )
            .child(
                label()
                    .text(format_time(self.duration))
                    .font_size(11.)
                    .max_lines(1),
            )
    }
}

#[derive(PartialEq)]
pub(crate) struct PlaybackTimes {
    pub position: u64,
    pub duration: u64,
    pub muted: bool,
}
impl Component for PlaybackTimes {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::Flex)
            .color(if self.muted {
                colors.muted_foreground
            } else {
                colors.foreground
            })
            .child(
                label()
                    .text(format_time(self.position))
                    .font_size(11.)
                    .width(Size::flex(1.))
                    .max_lines(1)
                    .text_overflow(TextOverflow::Ellipsis)
                    .text_align(TextAlign::Left),
            )
            .child(
                label()
                    .text(format_time(self.duration))
                    .font_size(11.)
                    .width(Size::flex(1.))
                    .max_lines(1)
                    .text_overflow(TextOverflow::Ellipsis)
                    .text_align(TextAlign::Right),
            )
    }
}
#[derive(PartialEq)]
pub(crate) struct PlaybackProgress {
    pub compact: bool,
}
impl Component for PlaybackProgress {
    fn render(&self) -> impl IntoElement {
        let playback = use_playback();
        let state = playback.snapshot();
        rect()
            .width(Size::fill())
            .spacing(if self.compact { 2. } else { 8. })
            .child(CompactSeek {
                position: state.current_position,
                duration: state.duration,
                enabled: playback.available() && state.path.is_some(),
            })
            .child(PlaybackTimes {
                position: state.current_position,
                duration: state.duration,
                muted: true,
            })
    }
}
