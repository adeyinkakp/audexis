use super::*;

pub(super) fn collection_heading(
    kind: Kind,
    current: &Option<Collection>,
    title: &str,
    colors: crate::ui::theme::AudexisColors,
) -> Option<Element> {
    if kind == Kind::Favorites {
        return Some(
                rect()
                    .horizontal()
                    .spacing(24.)
                    .padding((20., 0.))
                    .cross_align(Alignment::Center)
                    .child(
                        rect()
                            .width(Size::px(112.))
                            .height(Size::px(112.))
                            .corner_radius(16.)
                            .background(colors.primary)
                            .center()
                            .child(
                                SvgViewer::new(lucide::heart())
                                    .width(Size::px(48.))
                                    .height(Size::px(48.))
                                    .stroke(Color::WHITE),
                            ),
                    )
                    .child(
                        rect()
                            .spacing(12.)
                            .child(label().text("Made by you").color(colors.muted_foreground))
                            .child(heading("Favorites", 36.))
                            .child(
                                label()
                                    .text("Heart a song anywhere to keep it here. Unheart it to remove it.")
                                    .color(colors.muted_foreground),
                            ),
                    ).into_element(),
            );
    } else if let Some(collection) = &current {
        return Some(
            rect()
                .horizontal()
                .spacing(24.)
                .padding((20., 0.))
                .cross_align(Alignment::Center)
                .maybe_child((kind != Kind::Artists).then(|| Artwork {
                    file_id: collection.file_id.unwrap_or(0),
                    path: String::new(),
                    size: 160.,
                }))
                .child(
                    rect()
                        .spacing(12.)
                        .child(
                            label()
                                .text(kind.title().trim_end_matches('s').to_uppercase())
                                .font_size(12.)
                                .color(colors.muted_foreground),
                        )
                        .child(heading(&title, 36.))
                        .child(
                            label()
                                .text(format!("{} songs", collection.track_count))
                                .color(colors.muted_foreground),
                        ),
                )
                .into_element(),
        );
    } else if !matches!(kind, Kind::Playlists) {
        return Some(
            heading(
                if kind == Kind::Search {
                    "Search your library"
                } else {
                    "Your library"
                },
                30.,
            )
            .into_element(),
        );
    }
    None
}
