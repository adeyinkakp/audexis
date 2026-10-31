use super::Artwork;
use crate::ui::theme::use_audexis_colors;
use freya::prelude::*;

#[derive(PartialEq)]
pub(crate) struct ArtworkCard {
    pub file_id: i64,
    pub title: String,
    pub subtitle: String,
    pub size: f32,
    pub radius: f32,
}
impl Component for ArtworkCard {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        rect()
            .width(Size::px(self.size))
            .spacing(10.)
            .child(
                rect()
                    .width(Size::px(self.size))
                    .height(Size::px(self.size))
                    .corner_radius(self.radius)
                    .overflow(Overflow::Clip)
                    .child(Artwork {
                        file_id: self.file_id,
                        path: String::new(),
                        size: self.size,
                    }),
            )
            .child(
                label()
                    .text(crate::ui::display_text(self.title.clone()))
                    .max_lines(1)
                    .text_overflow(TextOverflow::Ellipsis),
            )
            .child(
                label()
                    .text(crate::ui::display_text(self.subtitle.clone()))
                    .font_size(12.)
                    .color(colors.muted_foreground),
            )
    }
}
