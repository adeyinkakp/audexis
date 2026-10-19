use super::*;
const PAGE_PADDING: f32 = 28.;
const CARD_PADDING: f32 = 8.;
const GRID_GAP: f32 = 20.;
#[derive(PartialEq)]
pub(super) struct LibraryPage {
    pub(super) kind: Kind,
    pub(super) initial: Option<Collection>,
}
impl Component for LibraryPage {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let library = use_library();
        let root_kind = self.kind;
        let mut active_kind = use_state(|| self.kind);
        let kind = *active_kind.read();
        let mut search = use_state(String::new);
        let mut detail = use_state(|| self.initial.clone());
        let mut creating = use_state(|| false);
        let mut name = use_state(String::new);
        let mut reload = use_state(|| 0u64);
        let mut width = use_state(|| 700f32);
        let key = (
            kind,
            search.read().clone(),
            detail.read().clone(),
            library.revision(),
            *reload.read(),
        );
        let db = services.db.clone();
        let load_runtime = runtime.clone();
        let (pages, load_more) = crate::ui::hooks::use_infinite_pages(
            key.clone(),
            move |(kind, query, detail, _, _), offset| {
                let db = db.clone();
                let runtime = load_runtime.clone();
                async move {
                    runtime
                        .spawn(async move {
                            let mut collections = Vec::new();
                            let mut next = None;
                            let tracks = if detail.is_some()
                                || matches!(kind, Kind::Favorites | Kind::Search)
                            {
                                let filter = TrackFilter {
                                    artist: detail.as_ref().and_then(|d| match kind {
                                        Kind::Artists => Some(d.name.clone()),
                                        Kind::Albums if !d.name.is_empty() => {
                                            Some(d.artist.clone())
                                        }
                                        _ => None,
                                    }),
                                    album: detail
                                        .as_ref()
                                        .filter(|_| kind == Kind::Albums)
                                        .map(|d| d.name.clone()),
                                    playlist: detail.as_ref().and_then(|d| d.playlist_id),
                                    favorites: kind == Kind::Favorites,
                                    search: query.clone(),
                                };
                                if kind == Kind::Artists {
                                    let page = browse_library_with_pool(
                                        &db.pool,
                                        CollectionKind::Albums,
                                        query.clone(),
                                        detail.as_ref().map(|d| d.name.clone()),
                                        offset,
                                    )
                                    .await?;
                                    collections = page.items;
                                    next = page.next_cursor;
                                }
                                if offset == 0 {
                                    db.collection_tracks(filter)
                                        .await
                                        .map_err(|e| e.to_string())?
                                } else {
                                    Vec::new()
                                }
                            } else {
                                let page = browse_library_with_pool(
                                    &db.pool,
                                    kind.collection(),
                                    query,
                                    None,
                                    offset,
                                )
                                .await?;
                                collections = page.items;
                                next = page.next_cursor;
                                Vec::new()
                            };
                            Ok::<_, String>(((collections, Arc::new(tracks)), next))
                        })
                        .await
                        .map_err(|e| e.to_string())?
                }
            },
        );
        let pages = pages.read();
        let collections = pages
            .pages
            .iter()
            .flat_map(|(collections, _)| collections.iter().cloned())
            .collect::<Vec<_>>();
        let tracks = pages
            .pages
            .first()
            .map(|(_, tracks)| tracks.clone())
            .unwrap_or_default();
        let result = if pages.pages.is_empty() {
            None
        } else {
            Some((&collections, &tracks))
        };
        let current = detail.read().clone();
        let min_card = if kind == Kind::Playlists { 170. } else { 150. };
        let available = (*width.read() - PAGE_PADDING * 2.).max(0.);
        let columns = ((available + GRID_GAP) / (min_card + CARD_PADDING * 2. + GRID_GAP))
            .floor()
            .max(1.) as usize;
        let card_size = ((available - (columns - 1) as f32 * GRID_GAP) / columns as f32
            - CARD_PADDING * 2.)
            .max(0.);
        let card_layout = ButtonLayoutThemePartial {
            padding: Some(Preference::Specific(Gaps::new_all(CARD_PADDING))),
            ..Default::default()
        };
        let title = current
            .as_ref()
            .map(|c| display_name(&c.name))
            .unwrap_or_else(|| kind.title().into());
        let mut body = rect()
            .width(Size::fill())
            .padding(PAGE_PADDING)
            .spacing(20.)
            .font_size(14.)
            .on_sized(move |event: Event<SizedEventData>| {
                let next = event.area.size.width as f32;
                if (*width.peek() - next).abs() > 1. {
                    width.set(next);
                }
            });
        let search_box =
            Input::new(search)
                .width(Size::px(300.))
                .placeholder(if current.is_some() {
                    format!(
                        "Search this {}",
                        kind.title().trim_end_matches('s').to_lowercase()
                    )
                } else {
                    format!("Search {}", kind.title().to_lowercase())
                });
        if kind == Kind::Playlists && current.is_none() {
            body = body
                .child(
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .content(Content::Flex)
                        .cross_align(Alignment::End)
                        .spacing(20.)
                        .child(
                            rect()
                                .width(Size::flex(1.))
                                .spacing(8.)
                                .child(
                                    label()
                                        .text("YOUR LIBRARY")
                                        .font_size(12.)
                                        .color(colors.muted_foreground),
                                )
                                .child(heading("Playlists", 36.)),
                        )
                        .child(search_box),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(1.))
                        .background(colors.border_subtle),
                );
        } else {
            body = body.child(search_box);
        }
        if current.is_some() {
            body = body.child(
                Button::new()
                    .flat()
                    .child(format!("‹ All {}", kind.title().to_lowercase()))
                    .on_press(move |_| {
                        active_kind.set(root_kind);
                        detail.set(None);
                        search.set(String::new());
                    }),
            );
        }
        if kind == Kind::Favorites {
            body = body.child(
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
                    ),
            );
        } else if let Some(collection) = &current {
            body = body.child(
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
                    ),
            );
        } else if !matches!(kind, Kind::Playlists) {
            body = body.child(heading(
                if kind == Kind::Search {
                    "Search your library"
                } else {
                    "Your library"
                },
                30.,
            ));
        }
        if kind == Kind::Search && current.is_none() {
            for kind in [Kind::Artists, Kind::Albums, Kind::Playlists] {
                body = body.child(SearchCollections {
                    kind,
                    query: search.read().clone(),
                    on_select: EventHandler::new(move |(kind, item)| {
                        active_kind.set(kind);
                        detail.set(Some(item));
                        search.set(String::new());
                    }),
                });
            }
        }
        if let Some(id) = current.as_ref().and_then(|c| c.playlist_id) {
            let refresh_library = library.clone();
            body = body.child(crate::ui::pages::playlist_actions::PlaylistActions {
                id,
                name: title.clone(),
                on_change: EventHandler::new(move |name: Option<String>| {
                    refresh_library.refresh();
                    if let Some(name) = name {
                        if let Some(current) = detail.write().as_mut() {
                            current.name = name;
                        }
                    } else {
                        detail.set(None);
                    }
                    *reload.write() += 1;
                }),
            });
        }
        if *creating.read() {
            let refresh_library = library.clone();
            body = body.child(
                rect()
                    .width(Size::fill())
                    .padding(20.)
                    .spacing(12.)
                    .corner_radius(12.)
                    .background(colors.surface_subtle)
                    .child(heading("Create Playlist", 20.))
                    .child(crate::ui::components::PlaylistForm {
                        name,
                        on_created: EventHandler::new(move |(id, title)| {
                            refresh_library.refresh();
                            creating.set(false);
                            name.set(String::new());
                            detail.set(Some(Collection {
                                name: title,
                                artist: String::new(),
                                file_id: None,
                                playlist_id: Some(id),
                                track_count: 0,
                            }));
                            *reload.write() += 1;
                        }),
                        on_cancel: Some(EventHandler::new(move |()| creating.set(false))),
                    }),
            );
        }
        match result {
            None => {}
            Some((collections, tracks)) => {
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
                    if key
                        .1
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
                                                        .stop((
                                                            Color::from_rgb(124, 58, 237),
                                                            100.,
                                                        )),
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
                if !cards.is_empty() {
                    if kind != Kind::Playlists {
                        body = body.child(heading(
                            if current.is_some() {
                                "Albums"
                            } else {
                                kind.title()
                            },
                            24.,
                        ));
                    }
                    for row in cards.chunks(columns) {
                        body = body.child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .spacing(GRID_GAP)
                                .children(row.iter().cloned()),
                        );
                    }
                }
                if current.is_none()
                    && !matches!(kind, Kind::Favorites | Kind::Search)
                    && collections.is_empty()
                {
                    body = body.child(
                        label()
                            .text(format!("No matching {}.", kind.title().to_lowercase()))
                            .color(colors.muted_foreground),
                    );
                }
                if current.is_some() || matches!(kind, Kind::Favorites | Kind::Search) {
                    body = body.child(TrackList {
                        tracks: tracks.clone(),
                        playlist: current.as_ref().and_then(|c| c.playlist_id),
                        on_change: EventHandler::new(move |()| *reload.write() += 1),
                    });
                }
            }
        }
        body = body.child(crate::ui::components::InfiniteScroll {
            has_more: pages.has_more(),
            loading: pages.loading,
            error: pages.error.clone(),
            progress: pages.pages.len(),
            on_load: load_more,
        });
        SmoothScroll::new().child(body.child(rect().height(Size::px(80.))))
    }
}
