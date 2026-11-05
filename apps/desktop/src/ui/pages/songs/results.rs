use super::*;

pub(super) fn results(
    data: Arc<Vec<SongRow>>,
    columns: Vec<audexis_core::config::TableColumn>,
    playback_query: SongQuery,
    width: f32,
    row_items: Arc<Vec<crate::ui::components::media::SelectedTrack>>,
    mut scroll: crate::ui::components::song_table::TableScroll,
    identity: String,
    length: usize,
    busy: State<bool>,
    search: State<String>,
    load_more: State<Callback<(), ()>>,
    colors: crate::ui::theme::AudexisColors,
) -> impl IntoElement {
    let mut viewport = scroll.viewport;
    let pointer_scroll = scroll.clone();
    let mut key_scroll = scroll.clone();
    let row_scroll = scroll.clone();
    let controller = scroll.controller;
    rect()
            .width(Size::fill())
            .height(Size::flex(1.0))
            .on_sized(move |event: Event<SizedEventData>| {
                viewport.set_if_modified((event.area.width(), event.area.height()));
            })
            .on_pointer_down(move |_| pointer_scroll.release_axis())
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                scroll.key(&event, true)
            })
            .on_global_key_up(move |event: Event<KeyboardEventData>| {
                key_scroll.key(&event, false)
            })
            .child(if data.is_empty() {
                rect()
                    .padding(24.0)
                    .child(
                        label()
                            .text(if *busy.read() {
                                "Loading songs…"
                            } else if search.read().is_empty() {
                                "No songs yet. Add a music folder or connect your cloud library in Settings."
                            } else {
                                "No matching songs."
                            })
                            .color(colors.muted_foreground),
                    )
                    .into_element()
            } else {
                rect()
                    .key(identity)
                    .expanded()
                    .child(
                        VisibilityViewport::new().child(
                            VirtualScrollView::new_with_data_controlled(
                                (data, columns, playback_query, width, row_items),
                                move |index,
                                      (songs, columns, query, width, items): &(
                                    Arc<Vec<SongRow>>,
                                    Vec<TableColumn>,
                                    SongQuery,
                                    f32,
                                    Arc<Vec<crate::ui::components::media::SelectedTrack>>,
                                )| {
                                    let wheel = row_scroll.clone();
                                    if let Some(song) = songs.get(index) {
                                        rect()
                                            .key(song.id)
                                            .width(Size::px(*width))
                                            .height(Size::px(ROW_HEIGHT))
                                            .on_wheel(
                                                move |event: Event<WheelEventData>| {
                                                    wheel.wheel(&event)
                                                },
                                            )
                                            .child(SongTableRow {
                                                columns: columns.clone(),
                                                song: song.clone(),
                                                query: query.clone(),
                                                index,
                                                item: items[index].clone(),
                                                load_more,
                                                near_end: index + 15 >= songs.len(),
                                                busy,
                                            })
                                            .into_element()
                                    } else {
                                        rect()
                                            .height(Size::px(ROW_HEIGHT))
                                            .width(Size::px(*width))
                                            .on_wheel(
                                                move |event: Event<WheelEventData>| {
                                                    wheel.wheel(&event)
                                                },
                                            )
                                            .child(label().text("Loading more songs…"))
                                            .into_element()
                                    }
                                },
                                controller,
                            )
                            .length(length)
                            .item_size(ROW_HEIGHT),
                        ),
                    )
                    .into_element()
            })
}
