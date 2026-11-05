use crate::ui::theme::AudexisColors;
use freya::{icons::lucide, prelude::*};

pub fn view(title: &'static str, message: &'static str, colors: AudexisColors) -> Element {
    rect()
        .width(Size::fill())
        .spacing(24.)
        .child(
            rect()
                .spacing(8.)
                .child(
                    label()
                        .text(title)
                        .font_size(20.)
                        .font_weight(FontWeight::BOLD),
                )
                .child(
                    label()
                        .text("Review backend errors and logs.")
                        .color(colors.muted_foreground),
                ),
        )
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .content(Content::Flex)
                .spacing(12.)
                .padding(20.)
                .corner_radius(16.)
                .background(colors.surface_subtle)
                .border(Border::new().width(1.).fill(colors.border))
                .child(
                    SvgViewer::new(lucide::file_warning())
                        .width(Size::px(18.))
                        .height(Size::px(18.))
                        .stroke(colors.primary),
                )
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .spacing(10.)
                        .child(label().text("Backend error logs").font_size(14.))
                        .child(
                            label()
                                .text(concat!(
                                    "View errors recorded by the database, metadata backend, ",
                                    "audio player, and file watcher."
                                ))
                                .font_size(12.)
                                .color(colors.muted_foreground),
                        )
                        .child(Button::new().filled().enabled(false).child("View logs"))
                        .child(
                            label()
                                .text(message)
                                .font_size(12.)
                                .color(colors.muted_foreground),
                        ),
                ),
        )
        .into_element()
}
