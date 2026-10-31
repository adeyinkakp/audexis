mod cards;
mod heading;
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
        use_side_effect_with_deps(&self.initial, move |initial| {
            if initial.is_some() {
                detail.set_if_modified(initial.clone());
            }
        });
        let mut creating = use_state(|| false);
        let mut name = use_state(String::new);
        let mut reload = use_state(|| 0u64);
        let mut width = use_state(|| 700f32);
        let key = (
            kind,
            search.read().clone(),
            detail.read().clone(),
            library.connection(),
        );
        let db = services.db.clone();
        let load_runtime = runtime.clone();
        let (pages, load_more) = crate::ui::hooks::use_infinite_pages(
            key.clone(),
            library.revision().wrapping_add(*reload.read()),
            move |(kind, query, detail, _), offset| {
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
        if let Some(header) = heading::collection_heading(kind, &current, &title, colors.clone()) {
            body = body.child(header);
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
                let cards = cards::collection_cards(
                    kind,
                    &current,
                    collections,
                    card_size,
                    card_layout,
                    &key.1,
                    creating,
                    active_kind,
                    detail,
                    search,
                    colors.clone(),
                );
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
