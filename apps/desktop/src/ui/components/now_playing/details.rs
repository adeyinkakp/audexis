use super::*;

#[derive(PartialEq)]
pub(super) struct TrackDetails {
    pub(super) id: i64,
    pub(super) path: Option<String>,
    pub(super) song_menu: EventHandler<Event<PressEventData>>,
}
impl Component for TrackDetails {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let song_menu = self.song_menu.clone();
        let metadata = crate::ui::hooks::use_track_metadata(self.id, self.path.as_deref());
        let subtitle = metadata.subtitle();
        let title = metadata.title;
        rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(8.0)
            .child(Artwork {
                file_id: self.id,
                path: self.path.clone().unwrap_or_default(),
                size: 36.0,
            })
            .child(
                rect()
                    .width(Size::flex(1.0))
                    .spacing(2.0)
                    .child(
                        label()
                            .text(crate::ui::display_text(title))
                            .width(Size::fill())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(14.0)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        label()
                            .text(crate::ui::display_text(subtitle))
                            .width(Size::fill())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(12.0)
                            .color(colors.muted_foreground),
                    ),
            )
            .child(crate::ui::components::FavoriteButton { id: self.id })
            .child(player_button_sized(
                "Song options",
                lucide::ellipsis(),
                self.path.is_some(),
                false,
                ControlSize::Bar,
                move |event| song_menu.call(event),
            ))
    }
}
