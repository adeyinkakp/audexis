use crate::ui::theme::AudexisColors;
use freya::prelude::*;

pub fn view(title: &'static str, message: &'static str, colors: AudexisColors) -> Element {
    rect()
        .spacing(18.0)
        .child(
            label()
                .text(title)
                .font_size(25.0)
                .font_weight(FontWeight::BOLD),
        )
        .child(
            rect()
                .padding(18.0)
                .background(colors.card)
                .corner_radius(12.0)
                .child(label().text(message).color(colors.muted_foreground)),
        )
        .into()
}
