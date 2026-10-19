use crate::ui::components::track_selection::{TrackSelectionScope, song_items};
use crate::{
    services::AppServices,
    ui::{
        components::{
            VisibilityViewport,
            song_table::{CustomFieldsEditor, TableHeader, use_table_scroll, use_table_settings},
        },
        hooks::use_library,
        theme::use_audexis_colors,
    },
};
use audexis_core::{
    config::TableColumn,
    database::songs::{SongQuery, SongRow},
};
use freya::prelude::*;
use std::sync::Arc;

use crate::ui::components::song_table::{ROW_HEIGHT, SongTableRow};

#[derive(PartialEq)]
pub struct Songs;
impl Component for Songs {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let search = use_state(String::new);
        let settings = use_table_settings();
        let config = settings.table.read().clone();
        let sorting = (config.sort.clone(), config.descending);
        let columns = config.columns.clone();

        let mut requested_columns: Vec<_> = columns
            .iter()
            .filter(|c| c.column.storage_key("").is_some())
            .map(|c| c.column.clone())
            .collect();
        requested_columns.sort_by_key(|c| format!("{c:?}"));
        let library = use_library();
        let key = (
            search.read().clone(),
            sorting,
            library.revision(),
            requested_columns,
        );
        let source = use_reactive(&key);
        let mut rows = use_state(|| Arc::new(Vec::<SongRow>::new()));
        let mut total = use_state(|| 0i64);
        let mut busy = use_state(|| true);
        let mut selecting_all = use_state(|| false);
        let mut error = use_state(|| None::<String>);
        let loading_services = services.clone();
        let mut loader = use_hook(move || {
            FutureTask::create(move || {
                let key = source.peek().clone();
                let offset = rows.peek().len();
                let db = loading_services.db.clone();
                let runtime = runtime.clone();
                async move {
                    let requested = key.clone();
                    let task = runtime.spawn(async move {
                        if offset == 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(180)).await;
                        }
                        db.songs_page_columns(
                            &requested.0,
                            requested.1.0,
                            requested.1.1,
                            offset,
                            &requested.3,
                        )
                        .await
                        .map_err(|e| e.to_string())
                    });
                    let _abort = AbortLoad(task.abort_handle());
                    let result = task.await.map_err(|e| e.to_string()).and_then(|r| r);
                    if *source.peek() != key {
                        return;
                    }
                    match result {
                        Ok(page) => {
                            total.set(page.total);
                            let mut accumulated = rows.write();
                            Arc::make_mut(&mut accumulated).extend(page.rows);
                            error.set(None);
                        }
                        Err(message) => error.set(Some(message)),
                    }
                    busy.set(false);
                }
            })
        });
        use_side_effect_with_deps(&key, move |_| {
            loader.cancel();
            selecting_all.set(false);
            rows.set(Arc::new(Vec::new()));
            total.set(0);
            error.set(None);
            busy.set(true);
            loader.start();
        });
        let load_more = use_hook(move || {
            State::create(Callback::new(move |()| {
                if !*busy.peek()
                    && error.peek().is_none()
                    && rows.peek().len() < *total.peek() as usize
                {
                    busy.set(true);
                    loader.start();
                }
            }))
        });
        use_side_effect(move || {
            if *selecting_all.read() && !*busy.read() && error.read().is_none() {
                if rows.read().len() < *total.read() as usize {
                    let mut more = load_more;
                    more.write().call(());
                } else {
                    selecting_all.set(false);
                }
            }
        });
        let playback_query = SongQuery {
            search: key.0.clone(),
            sort: key.1.0.clone(),
            descending: key.1.1,
        };
        let selection_key = format!("track-selection:{playback_query:?}");
        let data = rows.read().clone();
        let selection_items = song_items(&data, None);
        let row_items = selection_items.clone();
        let more = data.len() < *total.read() as usize;
        let length = data.len() + usize::from(more);
        let width = 50.0 + columns.iter().map(|col| col.width as f32).sum::<f32>();
        let mut scroll = use_table_scroll(width, length as f32 * ROW_HEIGHT);
        let mut position = scroll.position;
        use_side_effect_with_deps(&key, move |_| {
            let x = position.peek().0;
            position.set((x, 0));
        });
        let mut viewport = scroll.viewport;
        let controller = scroll.controller;
        let row_scroll = scroll.clone();
        let header_scroll = scroll.clone();
        let pointer_scroll = scroll.clone();
        let mut key_scroll = scroll.clone();
        let save_settings = settings.clone();
        let header = rect()
            .width(Size::fill())
            .height(Size::px(36.0))
            .on_wheel(move |event: Event<WheelEventData>| {
                header_scroll.wheel(&event);
                let mut controller = header_scroll.controller;
                let current = *header_scroll.position.peek();
                let (x, y) = header_scroll.deltas(&event);
                controller.scroll_to_x(current.0 + x.round() as i32);
                controller.scroll_to_y(current.1 + y.round() as i32);
                event.stop_propagation();
            })
            .child(TableHeader {
                settings: settings.clone(),
                width,
                offset: scroll.position.read().0,
            });
        let body = rect()
            .expanded()
            .content(Content::Flex)
            .padding(20.0)
            .spacing(12.0)
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .spacing(12.0)
                    .child(
                        rect()
                            .width(Size::flex(1.0))
                            .child(
                                label()
                                    .text("Songs")
                                    .font_size(28.0)
                                    .font_weight(FontWeight::BOLD),
                            )
                            .child(
                                label()
                                    .text(format!(
                                        "{} songs · {} loaded",
                                        *total.read(),
                                        data.len()
                                    ))
                                    .font_size(12.0)
                                    .color(colors.muted_foreground),
                            ),
                    )
                    .child(
                        Input::new(search)
                            .placeholder("Search songs, artists, albums…")
                            .width(Size::px(260.0)),
                    ),
            )
            .maybe_child(error.read().clone().map(|message| {
                rect()
                    .horizontal()
                    .spacing(10.0)
                    .child(label().text(message).color(colors.destructive))
                    .child(Button::new().compact().child("Retry").on_press(move |_| {
                        error.set(None);
                        busy.set(true);
                        loader.start();
                    }))
            }))
            .maybe_child(
                (*settings.editing_fields.read()).then(|| CustomFieldsEditor {
                    standalone: false,
                    settings: settings.clone(),
                }),
            )
            .maybe_child(settings.error.read().clone().map(|message| {
                rect()
                    .horizontal()
                    .spacing(8.0)
                    .child(label().text(message).color(colors.destructive))
                    .child(
                        Button::new()
                            .compact()
                            .child("Retry save")
                            .on_press(move |_| save_settings.save(&services)),
                    )
            }))
            .maybe_child(
                (*selecting_all.read())
                    .then(|| label().text("Selecting all matching songs…").font_size(12.)),
            )
            .child(header)
            .child(
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
                                        "No songs yet. Add a music folder from Home."
                                    } else {
                                        "No matching songs."
                                    })
                                    .color(colors.muted_foreground),
                            )
                            .into_element()
                    } else {
                        rect()
                            .key(format!("{key:?}"))
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
                    }),
            );
        rect()
            .expanded()
            .key(selection_key)
            .child(TrackSelectionScope {
                items: selection_items,
                child: body.into_element(),
                fill: true,
                on_select_all: Some(EventHandler::new(move |()| selecting_all.set(true))),
            })
    }
}
struct AbortLoad(tokio::task::AbortHandle);
impl Drop for AbortLoad {
    fn drop(&mut self) {
        self.0.abort();
    }
}
