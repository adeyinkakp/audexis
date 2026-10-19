use crate::ui::theme::use_audexis_colors;
use freya::{animation::*, prelude::*};

#[derive(PartialEq)]
pub(super) struct LyricText {
    pub text: String,
    pub active: bool,
    pub reduce_motion: bool,
}

impl Component for LyricText {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
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
        let blur_target = if self.active || *hovering.read() || self.reduce_motion {
            0.0f32
        } else {
            0.65
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
                    .scale(0.8 + 0.2 * amount)
                    .blur(if self.reduce_motion {
                        0.0
                    } else {
                        blur.read().value()
                    })
                    .opacity(if *hovering.read() {
                        1.0
                    } else {
                        0.75 + 0.25 * amount
                    })
                    .child(
                        label()
                            .text(crate::ui::display_text(self.text.clone()))
                            .width(Size::fill())
                            .font_size(24.0)
                            .font_weight(FontWeight::BOLD)
                            .color(if self.active {
                                colors.primary
                            } else {
                                colors.muted_foreground
                            }),
                    ),
            )
    }
}
