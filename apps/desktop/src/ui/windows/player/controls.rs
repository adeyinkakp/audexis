use crate::ui::components::player_controls::{
    ControlSize, PlaybackAction, PlaybackButton, PlaybackProgress,
};
use freya::prelude::*;

#[derive(PartialEq)]
pub(super) struct PlayerControls {
    pub compact: bool,
}
impl Component for PlayerControls {
    fn render(&self) -> impl IntoElement {
        let size = if self.compact {
            ControlSize::Mini
        } else {
            ControlSize::Expanded
        };
        let mut controls = rect()
            .width(Size::fill())
            .horizontal()
            .cross_align(Alignment::Center)
            .main_align(Alignment::Center)
            .spacing(if self.compact { 20. } else { 32. });
        for action in [
            PlaybackAction::Shuffle,
            PlaybackAction::Previous,
            PlaybackAction::Toggle,
            PlaybackAction::Next,
            PlaybackAction::Repeat,
        ] {
            controls = controls.child(PlaybackButton { action, size });
        }
        rect()
            .width(Size::fill())
            .spacing(if self.compact { 2. } else { 8. })
            .child(PlaybackProgress {
                compact: self.compact,
            })
            .child(controls)
    }
}
