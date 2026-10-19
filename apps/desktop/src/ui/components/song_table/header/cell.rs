use super::*;

#[derive(PartialEq)]
pub(super) struct HeaderCell {
    pub(super) settings: TableSettings,
    pub(super) col: TableColumn,
    pub(super) shift: f32,
}
impl Component for HeaderCell {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let mut drag = use_state(|| None::<(f64, u16)>);
        let HeaderDrag {
            drag: mut reorder,
            mut suppress_click,
        } = use_consume::<HeaderDrag>();
        let source = reorder
            .read()
            .as_ref()
            .filter(|d| d.active)
            .map(|d| d.column.clone());
        let reduced = services
            .user_config
            .lock()
            .map(|c| c.reduce_motion)
            .unwrap_or(true);
        let shift = use_drag_offset(self.shift, reduced, source.is_some());
        use_drop(|| Cursor::set(CursorIcon::default()));
        let mut table = self.settings.table;
        let column = self.col.column.clone();
        let title = self.settings.title(&column);
        let current = table.read();
        let caption = if current.sort == column {
            format!("{title} {}", if current.descending { "↓" } else { "↑" })
        } else {
            title.clone()
        };
        drop(current);
        let sort_col = column.clone();
        let resize_col = column.clone();
        let start_col = column.clone();
        let key_col = column.clone();
        let save_sort = self.settings.clone();
        let save_resize = self.settings.clone();
        let save_key = self.settings.clone();
        let menu_settings = self.settings.clone();
        let sort_services = services.clone();
        let resize_services = services.clone();
        let key_services = services.clone();
        let drag_column = column.clone();
        rect()
            .key(format!("{column:?}"))
            .width(Size::px(self.col.width as f32))
            .height(Size::px(36.0))
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .offset_x(shift)
            .interactive(source.is_none())
            .opacity(if source.as_ref() == Some(&self.col.column) {
                0.0
            } else {
                1.0
            })
            .on_secondary_down(move |event: Event<PressEventData>| {
                event.stop_propagation();
                event.prevent_default();
                menu::open(
                    &event,
                    Some(column.clone()),
                    menu_settings.clone(),
                    services.clone(),
                );
            })
            .child(
                rect()
                    .width(Size::flex(1.0))
                    .height(Size::fill())
                    .padding((0.0, 6.0))
                    .main_align(Alignment::Center)
                    .a11y_focusable(true)
                    .a11y_alt(format!("Sort by {title}; drag to reorder column"))
                    .on_pointer_down(move |event: Event<PointerEventData>| {
                        if !event.data().is_primary() {
                            return;
                        }
                        event.stop_propagation();
                        suppress_click.set(false);
                        reorder.set(Some(ColumnDrag {
                            column: drag_column.clone(),
                            start_x: event.global_location().x,
                            x: event.global_location().x,
                            y: event.global_location().y,
                            active: false,
                        }));
                    })
                    .on_press(move |event: Event<PressEventData>| {
                        if matches!(event.data(), PressEventData::Keyboard(_)) {
                            suppress_click.set(false);
                        }
                        if drag.peek().is_some()
                            || *suppress_click.peek()
                            || reorder.peek().as_ref().is_some_and(|d| d.active)
                        {
                            return;
                        }
                        {
                            let mut cfg = table.write();
                            cfg.descending = cfg.sort == sort_col && !cfg.descending;
                            cfg.sort = sort_col.clone();
                        }
                        save_sort.save(&sort_services);
                    })
                    .child(
                        label()
                            .text(caption)
                            .width(Size::fill())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(12.0)
                            .color(colors.muted_foreground),
                    ),
            )
            .child(
                rect()
                    .width(Size::px(8.0))
                    .height(Size::fill())
                    .center()
                    .on_pointer_enter(|_| Cursor::set(CursorIcon::ColResize))
                    .on_pointer_leave(move |_| {
                        if drag.peek().is_none() {
                            Cursor::set(CursorIcon::default());
                        }
                    })
                    .a11y_focusable(true)
                    .a11y_alt(format!(
                        "Resize {title}; left and right arrows adjust width"
                    ))
                    .on_pointer_down(move |event: Event<PointerEventData>| {
                        if !event.data().is_primary() {
                            return;
                        }
                        event.stop_propagation();
                        event.prevent_default();
                        let width = table
                            .peek()
                            .columns
                            .iter()
                            .find(|c| c.column == start_col)
                            .map(|c| c.width)
                            .unwrap_or(180);
                        drag.set(Some((event.global_location().x, width)));
                    })
                    .on_press(|event: Event<PressEventData>| {
                        event.stop_propagation();
                        event.prevent_default();
                    })
                    .on_global_pointer_move(move |event: Event<PointerEventData>| {
                        if let Some((x, width)) = *drag.peek() {
                            let width = (width as f64 + event.global_location().x - x)
                                .round()
                                .clamp(64.0, 1200.0) as u16;
                            let changed = table
                                .peek()
                                .columns
                                .iter()
                                .any(|c| c.column == resize_col && c.width != width);
                            if changed {
                                if let Some(col) = table
                                    .write()
                                    .columns
                                    .iter_mut()
                                    .find(|c| c.column == resize_col)
                                {
                                    col.width = width;
                                }
                            }
                            event.prevent_default();
                        }
                    })
                    .on_global_pointer_press(move |_| {
                        if drag.take().is_some() {
                            Cursor::set(CursorIcon::default());
                            save_resize.save(&resize_services);
                        }
                    })
                    .on_key_down(move |event: Event<KeyboardEventData>| {
                        let delta = match event.key {
                            Key::Named(NamedKey::ArrowLeft) => -16,
                            Key::Named(NamedKey::ArrowRight) => 16,
                            _ => return,
                        };
                        event.stop_propagation();
                        if let Some(col) = table
                            .write()
                            .columns
                            .iter_mut()
                            .find(|c| c.column == key_col)
                        {
                            col.width = (col.width as i32 + delta).clamp(64, 1200) as u16;
                        }
                        save_key.save(&key_services);
                    })
                    .child(
                        rect()
                            .width(Size::px(1.0))
                            .height(Size::px(16.0))
                            .background(colors.border),
                    ),
            )
    }
}
