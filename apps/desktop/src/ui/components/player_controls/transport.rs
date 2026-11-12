use super::{ControlSize, button::styled_button};
use crate::ui::{
    hooks::{PlaybackPanelView, use_playback},
    theme::use_audexis_colors,
};
use audexis_audio::{PlayerCmd, RepeatMode};
use freya::{icons::lucide, prelude::*};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PlaybackAction {
    Previous,
    Toggle,
    Next,
    Shuffle,
    Repeat,
    Panel(PlaybackPanelView),
}
#[derive(PartialEq)]
pub(crate) struct PlaybackButton {
    pub action: PlaybackAction,
    pub size: ControlSize,
}
impl Component for PlaybackButton {
    fn render(&self) -> impl IntoElement {
        let playback = use_playback();
        let state = playback.snapshot();
        let action = self.action;
        let size = self.size;
        let (title, icon, active) = match action {
            PlaybackAction::Previous => ("Previous", lucide::skip_back(), false),
            PlaybackAction::Toggle => (
                if state.paused { "Play" } else { "Pause" },
                if state.paused {
                    lucide::play()
                } else {
                    lucide::pause()
                },
                false,
            ),
            PlaybackAction::Next => ("Next", lucide::skip_forward(), false),
            PlaybackAction::Shuffle => (
                if state.shuffled {
                    "Disable shuffle"
                } else {
                    "Enable shuffle"
                },
                lucide::shuffle(),
                state.shuffled,
            ),
            PlaybackAction::Repeat => (
                match state.repeat_mode {
                    RepeatMode::Off => "Repeat: off",
                    RepeatMode::Queue => "Repeat: queue",
                    RepeatMode::Track => "Repeat: track",
                },
                if state.repeat_mode == RepeatMode::Track {
                    lucide::repeat_1()
                } else {
                    lucide::repeat()
                },
                state.repeat_mode != RepeatMode::Off,
            ),
            PlaybackAction::Panel(view) => (
                if view == PlaybackPanelView::Lyrics {
                    "Lyrics"
                } else {
                    "Queue"
                },
                if view == PlaybackPanelView::Lyrics {
                    lucide::message_square_text()
                } else {
                    lucide::list_music()
                },
                *playback.panel.read() == Some(view),
            ),
        };
        let enabled = matches!(action, PlaybackAction::Panel(_))
            || (playback.available() && state.path.is_some());
        let prominent = action == PlaybackAction::Toggle
            && matches!(size, ControlSize::Mini | ControlSize::Expanded);
        let colors = use_audexis_colors();
        let color = if prominent {
            colors.primary_foreground
        } else if active {
            colors.primary
        } else if size == ControlSize::Bar {
            colors.muted_foreground
        } else {
            colors.content_secondary
        };
        styled_button(
            title,
            SvgViewer::new(icon).color(color),
            enabled,
            size,
            prominent,
            move |_| match action {
                PlaybackAction::Previous => playback.send(PlayerCmd::Previous),
                PlaybackAction::Toggle => playback.send(if state.paused {
                    PlayerCmd::Play { resuming: true }
                } else {
                    PlayerCmd::Pause
                }),
                PlaybackAction::Next => playback.send(PlayerCmd::Next),
                PlaybackAction::Shuffle => playback.send(PlayerCmd::ToggleShuffle),
                PlaybackAction::Repeat => playback.send(PlayerCmd::CycleRepeat),
                PlaybackAction::Panel(view) => playback.toggle_panel(view),
            },
        )
    }
}
