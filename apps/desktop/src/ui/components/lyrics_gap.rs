use crate::ui::theme::use_audexis_colors;
use freya::{animation::*, prelude::*};

#[derive(PartialEq)]
pub(super) struct LyricsGap {
    pub active: bool,
    pub position: u64,
    pub start: u64,
    pub end: Option<u64>,
    pub reduce_motion: bool,
}

fn progress(position: u64, start: u64, end: Option<u64>) -> f32 {
    let elapsed = position.saturating_sub(start);
    match end.filter(|end| *end > start) {
        Some(end) => (elapsed as f64 / (end - start) as f64).clamp(0.0, 1.0) as f32,
        None => (elapsed % 2400) as f32 / 2400.0,
    }
}

impl Component for LyricsGap {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let reduced = self.reduce_motion;
        let visibility = use_reactive(&(self.active, reduced));
        let transition = use_animation_transition(visibility, |(from, _), (to, reduced)| {
            AnimNum::new(if from { 1.0 } else { 0.0 }, if to { 1.0 } else { 0.0 })
                .time(if reduced { 0 } else { 350 })
                .ease(Ease::InOut)
                .function(Function::Cubic)
        });
        let amount = if reduced {
            if self.active { 1.0 } else { 0.0 }
        } else {
            transition.read().value().clamp(0.0, 1.0)
        };
        let mut dots = rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .height(Size::px(31.0))
            .spacing(6.0)
            .opacity(amount)
            .transform_origin(TransformOrigin::left())
            .scale(if reduced { 1.0 } else { 0.85 + 0.15 * amount })
            .a11y_alt("Instrumental break");
        let progress = progress(self.position, self.start, self.end);
        for index in 0..3 {
            dots = dots.child(GapDot {
                fill: (progress * 3.0 - index as f32).clamp(0.0, 1.0),
                reduce_motion: self.reduce_motion,
                color: colors.primary,
            });
        }

        rect()
            .width(Size::fill())
            .height(Size::px(43.0 * amount))
            .overflow(Overflow::Clip)
            .maybe(amount > 0.0, |row| row.child(dots))
    }
}

#[derive(PartialEq)]
struct GapDot {
    fill: f32,
    reduce_motion: bool,
    color: Color,
}
impl Component for GapDot {
    fn render(&self) -> impl IntoElement {
        let target = use_reactive(&(self.fill, self.reduce_motion));
        let animation = use_animation_transition(target, |(from, _), (to, reduced)| {
            AnimNum::new(from, to)
                .time(if reduced { 0 } else { 150 })
                .function(Function::Linear)
        });
        let fill = if self.reduce_motion {
            self.fill
        } else {
            animation.read().value()
        };
        rect()
            .width(Size::px(8.0))
            .height(Size::px(8.0))
            .corner_radius(99.0)
            .background(self.color)
            .opacity(0.3 + fill * 0.7)
            .scale(if self.reduce_motion {
                1.0
            } else {
                1.0 + (fill * std::f32::consts::PI).sin() * 0.3
            })
    }
}
