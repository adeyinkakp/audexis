use super::super::compact_seek::CompactSeek;
use crate::ui::{format_time, hooks::use_playback, theme::use_audexis_colors};
use freya::prelude::*;

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
            .child(label().text(format_time(self.position)).font_size(11.))
            .child(rect().width(Size::flex(1.)))
            .child(label().text(format_time(self.duration)).font_size(11.))
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
        let hovering = use_state(|| false);
        rect()
            .width(Size::fill())
            .spacing(if self.compact { 2. } else { 8. })
            .child(CompactSeek {
                position: state.current_position,
                duration: state.duration,
                enabled: playback.available() && state.path.is_some(),
                hovering,
            })
            .child(PlaybackTimes {
                position: state.current_position,
                duration: state.duration,
                muted: true,
            })
    }
}
