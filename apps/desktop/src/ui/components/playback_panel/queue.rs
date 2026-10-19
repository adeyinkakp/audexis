use super::*;
use crate::ui::components::{
    media::{MediaTrack, SelectedTrack},
    track_selection::{SelectableTrack, TrackSelectionScope},
};
#[derive(PartialEq)]
pub(super) struct QueuePanel;
impl Component for QueuePanel {
    fn render(&self) -> impl IntoElement {
        super::super::queue_drag::provide_queue_drag();
        let playback = use_playback();
        let queue = playback.queue.read().clone();
        let length = queue.queue_ids.len();
        let colors = use_audexis_colors();
        let items = Arc::new(
            queue
                .queue_ids
                .iter()
                .enumerate()
                .map(|(index, id)| SelectedTrack {
                    track: MediaTrack {
                        id: queue.file_ids[index],
                        path: queue.paths[index].clone(),
                    },
                    queue_id: Some(id.clone()),
                    playlist: None,
                })
                .collect::<Vec<_>>(),
        );
        let body = rect()
            .expanded()
            .content(Content::Flex)
            .child(
                rect()
                    .width(Size::fill())
                    .padding(10.0)
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .child(
                        label()
                            .text(format!("{length} tracks"))
                            .font_size(12.0)
                            .color(colors.muted_foreground),
                    )
                    .child(rect().width(Size::flex(1.0)))
                    .child(
                        Button::new()
                            .compact()
                            .flat()
                            .enabled(length > 0)
                            .child("Clear")
                            .on_press(move |_| playback.send(PlayerCmd::ClearQueue)),
                    ),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.0))
                    .child(if length == 0 {
                        rect()
                            .padding(20.0)
                            .child(
                                label()
                                    .text("Your queue is empty. Play a song or a discovery shelf to start.")
                                    .color(colors.muted_foreground),
                            )
                            .into_element()
                    } else {
                        VisibilityViewport::new()
                            .child(
                                VirtualScrollView::new_with_data(queue, |index, queue: &QueueInfo| {
                                    rect()
                                        .key(queue.queue_ids[index].clone())
                                        .height(Size::px(68.0))
                                        .child(QueueRow {
                                            queue_id: queue.queue_ids[index].clone(),
                                            file_id: queue.file_ids[index],
                                            path: queue.paths[index].clone(),
                                            current: queue.current_index == index as i32,
                                        })
                                        .into_element()
                                })
                                .length(length)
                                .item_size(68.0),
                            )
                            .into_element()
                    }),
            );
        TrackSelectionScope {
            items,
            child: body.into_element(),
            fill: true,
            on_select_all: None,
        }
    }
}
#[derive(PartialEq)]
struct QueueRow {
    queue_id: String,
    file_id: i64,
    path: String,
    current: bool,
}
impl Component for QueueRow {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let playback = use_playback();
        let songs = use_songs(&[self.file_id]);
        let song = songs.data.as_ref().and_then(|s| s.first());
        let title = song.map(|s| s.title.clone()).unwrap_or_else(|| {
            Path::new(&self.path)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| self.path.clone())
        });
        let artist = song
            .map(|s| s.artist.clone())
            .unwrap_or_else(|| "Local audio".into());
        let play = playback.clone();
        let remove = playback.clone();
        let play_id = self.queue_id.clone();
        let remove_id = self.queue_id.clone();
        let dragging = use_drag::<super::super::queue_drag::QueueEntryDrag>();
        let drag_title = title.clone();
        let drag_artist = artist.clone();
        let row = SelectableTrack::new(
            SelectedTrack {
                track: MediaTrack {
                    id: self.file_id,
                    path: self.path.clone(),
                },
                queue_id: Some(self.queue_id.clone()),
                playlist: None,
            },
            MediaTarget::queue_track(self.file_id, self.path.clone(), self.queue_id.clone()),
            EventHandler::new(move |()| {
                if dragging.peek().is_none() {
                    play.send(PlayerCmd::PlayQueueEntry {
                        queue_id: play_id.clone(),
                    });
                }
            }),
        )
        .background(if self.current {
            colors.surface_subtle
        } else {
            Color::TRANSPARENT
        })
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(68.0))
                .padding(4.0)
                .horizontal()
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .child(
                    rect().width(Size::flex(1.0)).child(
                        rect().width(Size::fill()).padding((6., 8.)).child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .content(Content::Flex)
                                .cross_align(Alignment::Center)
                                .spacing(8.0)
                                .child(Artwork {
                                    file_id: self.file_id,
                                    path: self.path.clone(),
                                    size: 32.0,
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
                                                .font_size(12.0)
                                                .color(if self.current {
                                                    colors.primary
                                                } else {
                                                    colors.foreground
                                                }),
                                        )
                                        .child(
                                            label()
                                                .text(crate::ui::display_text(artist))
                                                .width(Size::fill())
                                                .max_lines(1)
                                                .text_overflow(TextOverflow::Ellipsis)
                                                .font_size(10.0)
                                                .color(colors.muted_foreground),
                                        ),
                                ),
                        ),
                    ),
                )
                .child(
                    rect()
                        .on_pointer_down(|event: Event<PointerEventData>| event.stop_propagation())
                        .child(icon_button(
                            "Remove from queue",
                            SvgViewer::new(("x", lucide::x())),
                            true,
                            move |event: Event<PressEventData>| {
                                event.stop_propagation();
                                if dragging.peek().is_some() {
                                    return;
                                }
                                remove.send(PlayerCmd::RemoveQueueEntry {
                                    queue_id: remove_id.clone(),
                                })
                            },
                        )),
                ),
        );
        super::super::queue_drag::QueueDragRow {
            queue_id: self.queue_id.clone(),
            title: drag_title,
            artist: drag_artist,
            file_id: self.file_id,
            path: self.path.clone(),
            child: row.into_element(),
        }
    }
}
