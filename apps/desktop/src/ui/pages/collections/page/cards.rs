use super::*;

pub(super) fn collection_cards(
    kind: Kind,
    current: &Option<Collection>,
    collections: &[Collection],
    card_size: f32,
    card_layout: ButtonLayoutThemePartial,
    query: &str,
    mut creating: State<bool>,
    mut active_kind: State<Kind>,
    mut detail: State<Option<Collection>>,
    mut search: State<String>,
    colors: crate::ui::theme::AudexisColors,
) -> Vec<Element> {
    let mut cards = Vec::new();
    if kind == Kind::Playlists && current.is_none() {
        cards.push(
            Button::new()
                .flat()
                .theme_layout(card_layout.clone())
                .child(
                    rect()
                        .width(Size::px(card_size))
                        .spacing(12.)
                        .child(
                            rect()
                                .width(Size::px(card_size))
                                .height(Size::px(card_size))
                                .center()
                                .corner_radius(12.)
                                .background(colors.surface_subtle)
                                .border(Border::new().fill(colors.border).width(1.))
                                .child(
                                    SvgViewer::new(lucide::plus())
                                        .width(Size::px(64.))
                                        .height(Size::px(64.))
                                        .stroke(colors.primary),
                                ),
                        )
                        .child(label().text("Create Playlist"))
                        .child(
                            label()
                                .text("Make something yours")
                                .font_size(12.)
                                .color(colors.muted_foreground),
                        ),
                )
                .on_press(move |_| creating.set(true))
                .into_element(),
        );
        if query
            .split_whitespace()
            .all(|t| "favorites".contains(&t.to_lowercase()))
        {
            cards.push(
                Link::new(crate::ui::routes::Route::Favorites)
                    .child(
                        rect()
                            .width(Size::px(card_size + CARD_PADDING * 2.))
                            .padding(CARD_PADDING)
                            .spacing(12.)
                            .child(
                                rect()
                                    .width(Size::px(card_size))
                                    .height(Size::px(card_size))
                                    .center()
                                    .corner_radius(12.)
                                    .background(
                                        LinearGradient::new()
                                            .angle(135.)
                                            .stop((Color::from_rgb(251, 113, 133), 0.))
                                            .stop((Color::from_rgb(236, 72, 153), 50.))
                                            .stop((Color::from_rgb(124, 58, 237), 100.)),
                                    )
                                    .child(
                                        SvgViewer::new(lucide::heart())
                                            .width(Size::px(64.))
                                            .height(Size::px(64.))
                                            .stroke(Color::WHITE),
                                    ),
                            )
                            .child(label().text("Favorites"))
                            .child(
                                label()
                                    .text("The songs you love")
                                    .font_size(12.)
                                    .color(colors.muted_foreground),
                            ),
                    )
                    .into_element(),
            );
        }
    }
    for collection in collections {
        let target = collection.clone();
        let artist_albums = kind == Kind::Artists && current.is_some();
        let round = kind == Kind::Artists && current.is_none();
        cards.push(
            Button::new()
                .flat()
                .theme_layout(card_layout.clone())
                .child(crate::ui::components::ArtworkCard {
                    file_id: collection.file_id.unwrap_or(0),
                    title: display_name(&collection.name),
                    subtitle: if !round && kind != Kind::Playlists {
                        collection.artist.clone()
                    } else {
                        format!("{} songs", collection.track_count)
                    },
                    size: card_size,
                    radius: if round { card_size / 2. } else { 12. },
                })
                .on_press(move |_| {
                    if artist_albums {
                        active_kind.set(Kind::Albums);
                    }
                    detail.set(Some(target.clone()));
                    search.set(String::new());
                })
                .into_element(),
        );
    }
    cards
}
