use freya::{prelude::*, text_edit::*};

#[derive(PartialEq)]
pub struct TextArea {
    pub value: Writable<String>,
    pub enabled: bool,
    pub height: f32,
    pub on_change: EventHandler<String>,
}
impl Component for TextArea {
    fn render(&self) -> impl IntoElement {
        let id = use_hook(AccessibilityId::new_unique);
        let focus = use_focus(id);
        let holder = use_state(ParagraphHolder::default);
        let mut area = use_state(Area::default);
        let mut dragging = use_state(|| false);
        let mut editable = use_editable(|| self.value.read().clone(), EditableConfig::new);
        let mut value = self.value.clone();
        let on_change = self.on_change.clone();
        if *value.read() != editable.editor().read().committed_text() {
            let mut editor = editable.editor_mut().write();
            editor.set(&value.read());
            editor.clear_selection();
        }
        let text = editable.editor().read().rope().to_string();
        let cursor = if focus() != Focus::Not {
            Some(editable.editor().read().cursor_pos())
        } else {
            None
        };
        let selection = editable.editor().read().selection().clone();
        let selection = match selection {
            TextSelection::Range { from, to } => Some((from, to)),
            _ => None,
        };
        let colors = super::super::theme::use_audexis_colors();
        rect()
            .width(Size::fill())
            .height(Size::px(self.height))
            .padding(16.0)
            .background(colors.background)
            .font_size(14.)
            .border(Border::new().fill(colors.border).width(1.))
            .corner_radius(12.0)
            .a11y_id(id)
            .a11y_focusable(self.enabled)
            .a11y_role(AccessibilityRole::TextInput)
            .maybe(self.enabled, |el| {
                el.on_key_down(move |event: Event<KeyboardEventData>| {
                    event.stop_propagation();
                    editable.process_event(EditableEvent::KeyDown {
                        key: &event.key,
                        modifiers: event.modifiers,
                    });
                    let text = editable.editor().read().committed_text();
                    if *value.peek() != text {
                        value.set(text.clone());
                        on_change.call(text);
                    }
                })
                .on_global_pointer_press(move |_: Event<PointerEventData>| {
                    dragging.set(false);
                    editable.process_event(EditableEvent::Release);
                })
                .on_global_pointer_move(move |event: Event<PointerEventData>| {
                    if *dragging.peek() {
                        editable.process_event(EditableEvent::Move {
                            location: (event.global_location() - area.peek().to_f64().min())
                                .to_point(),
                            editor_line: EditorLine::SingleParagraph,
                            holder: &holder.read(),
                        });
                    }
                })
                .on_key_up(move |event: Event<KeyboardEventData>| {
                    editable.process_event(EditableEvent::KeyUp { key: &event.key })
                })
                .on_ime_preedit(move |event: Event<ImePreeditEventData>| {
                    editable
                        .editor_mut()
                        .write()
                        .set_preedit(&event.data().text);
                })
            })
            .child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::fill())
                    .child(
                        paragraph()
                            .width(Size::fill())
                            .min_height(Size::px((self.height - 32.).max(48.)))
                            .holder(holder.read().clone())
                            .span(text)
                            .cursor_index(cursor)
                            .cursor_color(colors.foreground)
                            .highlights(selection.map(|r| vec![r]))
                            .on_sized(move |event: Event<SizedEventData>| area.set(event.area))
                            .on_focus_press(move |event: Event<FocusPressEventData>| {
                                event.stop_propagation();
                                id.request_focus();
                                dragging.set(true);
                                editable.process_event(EditableEvent::Down {
                                    location: (event.global_location()
                                        - area.peek().to_f64().min())
                                    .to_point(),
                                    editor_line: EditorLine::SingleParagraph,
                                    holder: &holder.read(),
                                });
                            }),
                    ),
            )
    }
}
