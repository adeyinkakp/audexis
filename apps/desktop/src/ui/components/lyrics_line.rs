use crate::ui::theme::use_audexis_colors;
use freya::{animation::*, prelude::*};

#[derive(PartialEq)]
pub(super) struct LyricText {
    pub text: String,
    pub active: bool,
    pub distance: usize,
    pub following: bool,
    pub reduce_motion: bool,
}

impl Component for LyricText {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let expanded =
            use_try_consume::<crate::ui::windows::player::ExpandedPlayerContext>().is_some();
        let mut hovering = use_state(|| false);
        let target = if self.active { 1.0f32 } else { 0.0 };
        let state = use_reactive(&(target, self.reduce_motion));
        let emphasis = use_animation_transition(state, |(from, _), (to, reduced)| {
            AnimNum::new(from, to)
                .time(if reduced { 0 } else { 420 })
                .ease(Ease::InOut)
                .function(Function::Cubic)
        });
        let amount = if self.reduce_motion {
            target
        } else {
            emphasis.read().value()
        };
        let blur_target =
            if self.active || !self.following || *hovering.read() || self.reduce_motion {
                0.0f32
            } else {
                (self.distance.saturating_sub(1) as f32 * 0.65).min(2.0)
            };
        let blur_state = use_reactive(&(blur_target, self.reduce_motion));
        let blur = use_animation_transition(blur_state, |(from, _), (to, reduced)| {
            AnimNum::new(from, to)
                .time(if reduced { 0 } else { 420 })
                .ease(Ease::InOut)
                .function(Function::Cubic)
        });

        rect()
            .width(Size::fill())
            .on_pointer_enter(move |_| hovering.set_if_modified(true))
            .on_pointer_leave(move |_| hovering.set_if_modified(false))
            .child(
                rect()
                    .width(Size::fill())
                    .transform_origin(TransformOrigin::left())
                    .scale(if self.reduce_motion {
                        1.0
                    } else {
                        0.94 + 0.06 * amount
                    })
                    .blur(if self.reduce_motion {
                        0.0
                    } else {
                        blur.read().value()
                    })
                    .opacity(if self.active {
                        1.0
                    } else if *hovering.read() {
                        0.78
                    } else if !self.following {
                        0.6
                    } else {
                        let resting = match self.distance {
                            0 | 1 => 0.52,
                            2 => 0.34,
                            _ => 0.22,
                        };
                        resting + (1.0 - resting) * amount
                    })
                    .child(
                        label()
                            .text(crate::ui::display_text(self.text.clone()))
                            .width(Size::fill())
                            .font_size(if expanded { 42.0 } else { 27.0 })
                            .font_weight(FontWeight::BOLD)
                            .color(if self.active {
                                colors.primary
                            } else {
                                colors.foreground
                            }),
                    ),
            )
    }
}
