use super::super::sortable_motion::use_drag_offset;
use super::{TableSettings, menu};
use crate::{services::AppServices, ui::theme::use_audexis_colors};
use audexis_core::config::{SongColumn, TableColumn};
use freya::prelude::*;
use std::sync::Arc;

#[derive(Clone, PartialEq)]
struct ColumnDrag {
    column: SongColumn,
    start_x: f64,
    x: f64,
    y: f64,
    active: bool,
}
#[derive(Clone, Copy)]
struct HeaderDrag {
    drag: State<Option<ColumnDrag>>,
    suppress_click: State<bool>,
}

#[derive(PartialEq)]
pub struct TableHeader {
    pub settings: TableSettings,
    pub offset: i32,
    pub width: f32,
}
impl Component for TableHeader {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let settings = self.settings.clone();
        let services = use_consume::<Arc<AppServices>>();
        let cols = settings.table.read().columns.clone();
        let mut drag = use_state(|| None::<ColumnDrag>);
        let mut suppress_click = use_state(|| false);
        use_provide_context(move || HeaderDrag {
            drag,
            suppress_click,
        });
        let mut bounds = use_state(|| (0.0f64, 0.0f64, 0.0f64));
        let offset = use_reactive(&self.offset);
        let mut table = settings.table;
        let save = settings.clone();
        let save_services = services.clone();
        let active = drag.read().clone().filter(|d| d.active);
        let target = active
            .as_ref()
            .and_then(|d| target_column(d, &cols, *bounds.peek(), *offset.peek()));
        let from = active
            .as_ref()
            .and_then(|d| cols.iter().position(|c| c.column == d.column));
        let to = target
            .as_ref()
            .and_then(|t| cols.iter().position(|c| &c.column == t));
        let source_width = from.map(|i| cols[i].width as f32).unwrap_or(0.0);
        let gap = from.zip(to).map(|(from, to)| {
            let edge = if from < to { to + 1 } else { to };
            42.0 + self.offset as f32 + cols[..edge].iter().map(|c| c.width as f32).sum::<f32>()
                - if from < to { source_width } else { 0.0 }
        });
        rect()
            .width(Size::fill())
            .height(Size::px(36.0))
            .overflow(Overflow::Clip)
            .background(colors.card)
            .on_sized(move |event: Event<SizedEventData>| {
                bounds.set_if_modified((
                    event.area.min_x() as f64,
                    event.area.min_y() as f64,
                    event.area.width() as f64,
                ));
            })
            .on_global_pointer_move(move |event: Event<PointerEventData>| {
                let Some(mut next) = drag.peek().clone() else {
                    return;
                };
                next.x = event.global_location().x;
                next.y = event.global_location().y;
                next.active |= (next.x - next.start_x).abs() >= 6.0;
                if next.active {
                    Cursor::set(CursorIcon::Grabbing);
                }
                drag.set(Some(next));
            })
            .on_capture_global_pointer_press(move |event: Event<PointerEventData>| {
                let Some(mut finished) = drag.take() else {
                    return;
                };
                if !finished.active {
                    return;
                }
                finished.x = event.global_location().x;
                finished.y = event.global_location().y;
                suppress_click.set(true);
                event.prevent_default();
                Cursor::set(CursorIcon::default());
                let target = target_column(
                    &finished,
                    &table.peek().columns,
                    *bounds.peek(),
                    *offset.peek(),
                );
                if let Some(target) = target {
                    let changed = table.write().move_column(&finished.column, &target);
                    if changed {
                        save.save(&save_services);
                    }
                }
            })
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                if event.key == Key::Named(NamedKey::Escape) && drag.peek().is_some() {
                    drag.set(None);
                    suppress_click.set(true);
                    Cursor::set(CursorIcon::default());
                }
            })
            .on_secondary_down(move |event: Event<PressEventData>| {
                event.stop_propagation();
                event.prevent_default();
                menu::open(&event, None, settings.clone(), services.clone());
            })
            .child(
                rect()
                    .width(Size::px(self.width))
                    .height(Size::px(36.0))
                    .offset_x(self.offset as f32)
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .padding((0.0, 8.0))
                    .child(rect().width(Size::px(34.0)))
                    .children(cols.iter().cloned().enumerate().map(|(index, col)| {
                        rect()
                            .key(format!("{:?}", col.column))
                            .width(Size::px(col.width as f32))
                            .height(Size::px(36.0))
                            .child(HeaderCell {
                                settings: self.settings.clone(),
                                col,
                                shift: match (from, to) {
                                    (Some(from), Some(to))
                                        if from < to && index > from && index <= to =>
                                    {
                                        -source_width
                                    }
                                    (Some(from), Some(to))
                                        if from > to && index >= to && index < from =>
                                    {
                                        source_width
                                    }
                                    _ => 0.0,
                                },
                            })
                            .into_element()
                    })),
            )
            .maybe_child(gap.map(|x| {
                rect()
                    .position(Position::new_absolute().top(3.0))
                    .offset_x(x + 2.0)
                    .width(Size::px((source_width - 4.0).max(0.0)))
                    .height(Size::px(30.0))
                    .corner_radius(4.0)
                    .background(colors.primary_washed)
                    .interactive(false)
            }))
            .maybe_child(active.map(|d| {
                rect()
                    .position(Position::new_absolute().top(0.0))
                    .offset_x(
                        42.0 + self.offset as f32
                            + cols[..from.unwrap_or(0)]
                                .iter()
                                .map(|c| c.width as f32)
                                .sum::<f32>()
                            + (d.x - d.start_x) as f32,
                    )
                    .width(Size::px(source_width))
                    .height(Size::px(36.0))
                    .padding((0.0, 8.0))
                    .main_align(Alignment::Center)
                    .background(colors.popover)
                    .corner_radius(5.0)
                    .border(Border::new().width(1.0).fill(colors.border))
                    .interactive(false)
                    .child(
                        label()
                            .text({
                                let cfg = self.settings.table.read();
                                let title = self.settings.title(&d.column);
                                if cfg.sort == d.column {
                                    format!("{title} {}", if cfg.descending { "↓" } else { "↑" })
                                } else {
                                    title
                                }
                            })
                            .width(Size::fill())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(12.0)
                            .color(colors.foreground),
                    )
            }))
    }
}

fn target_column(
    drag: &ColumnDrag,
    columns: &[TableColumn],
    (left, top, width): (f64, f64, f64),
    offset: i32,
) -> Option<SongColumn> {
    if drag.x < left || drag.x > left + width || drag.y < top - 48.0 || drag.y > top + 84.0 {
        return None;
    }
    let from = columns.iter().position(|c| c.column == drag.column)?;
    let origin = left + offset as f64 + 42.0;
    let center = origin
        + columns[..from].iter().map(|c| c.width as f64).sum::<f64>()
        + columns[from].width as f64 / 2.0
        + drag.x
        - drag.start_x;
    let mut edge = origin;
    let mut target = from;
    for (index, col) in columns.iter().enumerate() {
        let midpoint = edge + col.width as f64 / 2.0;
        if index < from && center < midpoint {
            target = target.min(index);
        }
        if index > from && center > midpoint {
            target = target.max(index);
        }
        edge += col.width as f64;
    }
    Some(columns[target].column.clone())
}

mod cell;
use cell::HeaderCell;
