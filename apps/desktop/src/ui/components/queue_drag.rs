use super::sortable_motion::use_drag_offset;
use crate::ui::{hooks::use_playback, theme::use_audexis_colors};
use audexis_audio::PlayerCmd;
use freya::prelude::*;
use std::sync::Arc;

#[derive(Clone, Copy)]
struct QueueDropTarget(State<Option<(String, String)>>);

pub(super) fn provide_queue_drag() {
    use_provide_context(|| QueueDropTarget(State::create(None)));
}

#[derive(Clone, PartialEq)]
pub(super) struct QueueEntryDrag {
    pub queue_id: String,
}

#[derive(PartialEq)]
pub(super) struct QueueDragRow {
    pub queue_id: String,
    pub title: String,
    pub artist: String,
    pub file_id: i64,
    pub path: String,
    pub child: Element,
}
impl Component for QueueDragRow {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let playback = use_playback();
        let mut drag = use_drag::<QueueEntryDrag>();
        let QueueDropTarget(mut drop_target) = use_consume::<QueueDropTarget>();
        let services = use_consume::<Arc<crate::services::AppServices>>();
        let reduced = services
            .user_config
            .lock()
            .map(|c| c.reduce_motion)
            .unwrap_or(true);
        let source = drag.read().as_ref().map(|d| d.queue_id.clone());
        use_side_effect_with_deps(&source, move |source| {
            if source.is_none() {
                drop_target.set_if_modified(None);
            }
        });
        let is_source = source.as_ref() == Some(&self.queue_id);
        let destination = drop_target
            .read()
            .as_ref()
            .filter(|(owner, _)| Some(owner) == source.as_ref())
            .map(|(_, target)| target.clone());
        let is_target = destination.as_ref() == Some(&self.queue_id) && !is_source;
        let target = self.queue_id.clone();
        let hover_target = target.clone();
        let cleanup_id = self.queue_id.clone();
        use_drop(move || {
            if drag
                .peek()
                .as_ref()
                .is_some_and(|d| d.queue_id == cleanup_id)
            {
                drag.set(None);
                drop_target.set(None);
            }
        });
        let queue = playback.queue.read();
        let from = source
            .as_ref()
            .and_then(|id| queue.queue_ids.iter().position(|q| q == id));
        let to = destination
            .as_ref()
            .and_then(|id| queue.queue_ids.iter().position(|q| q == id));
        let index = queue.queue_ids.iter().position(|q| q == &self.queue_id);
        let shift = match (from, to, index) {
            (Some(from), Some(to), Some(index)) if from < to && index > from && index <= to => {
                -68.0
            }
            (Some(from), Some(to), Some(index)) if from > to && index >= to && index < from => 68.0,
            _ => 0.0,
        };
        drop(queue);
        let offset = use_drag_offset(shift, reduced, source.is_some());

        let content = rect()
            .width(Size::fill())
            .height(Size::px(68.0))
            .background(if is_target {
                colors.primary_washed
            } else {
                Color::TRANSPARENT
            })
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(68.0))
                    .offset_y(offset)
                    .interactive(source.is_none())
                    .opacity(if is_source { 0.0 } else { 1.0 })
                    .child(self.child.clone()),
            );
        let preview = rect()
            .width(Size::px(260.0))
            .padding(8.0)
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(10.0)
            .corner_radius(8.0)
            .background(colors.popover)
            .border(Border::new().width(1.0).fill(colors.primary))
            .child(super::Artwork {
                file_id: self.file_id,
                path: self.path.clone(),
                size: 40.0,
            })
            .child(
                rect()
                    .width(Size::flex(1.0))
                    .spacing(3.0)
                    .child(
                        label()
                            .text(self.title.clone())
                            .width(Size::fill())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(12.0)
                            .color(colors.foreground),
                    )
                    .child(
                        label()
                            .text(self.artist.clone())
                            .width(Size::fill())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(10.0)
                            .color(colors.muted_foreground),
                    ),
            );
        DropZone::new(
            DragZone::new(
                QueueEntryDrag {
                    queue_id: self.queue_id.clone(),
                },
                content,
            )
            .drag_threshold(6.0)
            .drag_element(preview),
            move |source: QueueEntryDrag| {
                drop_target.set(None);
                if source.queue_id == target {
                    return;
                }
                let queue = playback.queue.peek();
                let valid =
                    queue.queue_ids.contains(&source.queue_id) && queue.queue_ids.contains(&target);
                drop(queue);
                if valid {
                    playback.send(PlayerCmd::ReorderQueueEntry {
                        queue_id: source.queue_id,
                        target_id: target.clone(),
                    });
                }
            },
        )
        .on_drag_over(move |hovered| {
            if hovered {
                let source = drag.peek().as_ref().map(|d| d.queue_id.clone());
                if let Some(source) = source {
                    drop_target.set_if_modified(Some((source, hover_target.clone())));
                }
            }
        })
    }
}
