use super::*;

#[derive(PartialEq)]
pub(super) struct SearchCollections {
    pub(super) kind: Kind,
    pub(super) query: String,
    pub(super) on_select: EventHandler<(Kind, Collection)>,
}
impl Component for SearchCollections {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let library = use_library();
        let kind = self.kind;
        let key = (kind, self.query.clone(), library.revision());
        let (pages, load_more) =
            crate::ui::hooks::use_infinite_pages(key, move |(kind, query, _), cursor| {
                let db = services.db.clone();
                let runtime = runtime.clone();
                async move {
                    let page = runtime
                        .spawn(async move {
                            browse_library_with_pool(
                                &db.pool,
                                kind.collection(),
                                query,
                                None,
                                cursor,
                            )
                            .await
                        })
                        .await
                        .map_err(|e| e.to_string())??;
                    Ok((page.items, page.next_cursor))
                }
            });
        let pages = pages.read();
        let items = pages.pages.iter().flatten().collect::<Vec<_>>();
        let mut body = rect()
            .width(Size::fill())
            .spacing(16.)
            .child(heading(kind.title(), 24.));
        if !pages.pages.is_empty() && items.is_empty() {
            body = body.child(
                label()
                    .text(format!("No matching {}.", kind.title().to_lowercase()))
                    .color(colors.muted_foreground),
            );
        } else {
            body = body.child(
                ScrollView::new()
                    .direction(Direction::Horizontal)
                    .height(Size::px(240.))
                    .child(
                        rect().horizontal().spacing(20.).children(
                            items
                                .iter()
                                .map(|item| {
                                    let selected = (*item).clone();
                                    let callback = self.on_select.clone();
                                    Button::new()
                                        .flat()
                                        .child(crate::ui::components::ArtworkCard {
                                            file_id: item.file_id.unwrap_or(0),
                                            title: display_name(&item.name),
                                            subtitle: format!("{} songs", item.track_count),
                                            size: 150.,
                                            radius: if kind == Kind::Artists { 75. } else { 12. },
                                        })
                                        .on_press(move |_| callback.call((kind, selected.clone())))
                                        .into_element()
                                })
                                .chain(std::iter::once(
                                    rect()
                                        .width(Size::px(220.))
                                        .height(Size::px(200.))
                                        .child(crate::ui::components::InfiniteScroll {
                                            has_more: pages.has_more(),
                                            loading: pages.loading,
                                            error: pages.error.clone(),
                                            progress: pages.pages.len(),
                                            on_load: load_more,
                                        })
                                        .into_element(),
                                )),
                        ),
                    ),
            );
        }
        body.into_element()
    }
    fn render_key(&self) -> DiffKey {
        (&(self.kind as u8)).into()
    }
}
