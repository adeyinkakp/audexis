use super::super::*;

#[derive(PartialEq)]
pub struct Details;
impl Component for Details {
    fn render(&self) -> impl IntoElement {
        let session = use_consume::<State<Session>>();
        let services = use_consume::<Arc<AppServices>>();
        let selected = services
            .user_config
            .lock()
            .map(|c| c.metadata_fields.clone())
            .unwrap_or_default();
        let s = session.read();
        let colors = crate::ui::theme::use_audexis_colors();
        let runtime = use_consume::<Handle>();
        let fields: Vec<_> = field_catalog::catalog(Some(&s.files[0].original.tag_format))
            .into_iter()
            .filter(|f| f.editable && selected.contains(&f.storage_key))
            .collect();
        let mut body = rect()
            .width(Size::fill())
            .spacing(20.)
            .child(
                Button::new()
                    .outline()
                    .child("Choose Details fields…")
                    .on_press(move |_| {
                        crate::ui::launch_settings(services.clone(), runtime.clone());
                    }),
            )
            .child(crate::ui::track_info::lookup::Lookup { kind: Tab::Details });
        for pair in fields.chunks(2) {
            let mut row = rect()
                .horizontal()
                .width(Size::fill())
                .content(Content::Flex)
                .spacing(20.);
            for field in pair {
                row = row.child(rect().width(Size::flex(1.)).child(TextField {
                    field: field.key,
                    title: field.label.into(),
                    multi: field.multi_value,
                }));
            }
            if pair.len() == 1 {
                row = row.child(rect().width(Size::flex(1.)));
            }
            body = body.child(row);
        }
        body.child(
            rect()
                .width(Size::fill())
                .padding(16.)
                .spacing(8.)
                .corner_radius(12.)
                .background(colors.surface_subtle)
                .border(Border::new().fill(colors.border).width(1.))
                .child(
                    label()
                        .text(if s.files.len() == 1 {
                            "File".into()
                        } else {
                            format!("{} selected files", s.files.len())
                        })
                        .font_size(12.)
                        .color(colors.muted_foreground),
                )
                .children(s.files.iter().map(|file| {
                    label()
                        .text(file.original.path.display().to_string())
                        .font_size(12.)
                        .into_element()
                })),
        )
    }
}
#[derive(PartialEq)]
struct TextField {
    field: FrameKey,
    title: String,
    multi: bool,
}
impl Component for TextField {
    fn render(&self) -> impl IntoElement {
        let mut session = use_consume::<State<Session>>();
        let s = session.read().clone();
        let values = s.values(self.field);
        let field = self.field;
        let mut rows = rect().width(Size::fill()).spacing(6.).child(
            label()
                .font_size(12.)
                .color(crate::ui::theme::use_audexis_colors().muted_foreground)
                .text(if s.mixed(field) {
                    format!("{} · Mixed values (showing first track)", self.title)
                } else {
                    self.title.clone()
                }),
        );
        for index in 0..values.len().max(1) {
            rows = rows.child(TextValue {
                field,
                index,
                mixed: s.mixed(field),
                removable: self.multi || values.len() > 1,
            });
        }
        rows.maybe_child(self.multi.then(|| {
            Button::new()
                .enabled(!s.busy)
                .compact()
                .flat()
                .child("+ Add value")
                .on_press(move |_| {
                    let mut s = session.write();
                    let mut values = s.values(field);
                    values.push(TagValue::Text(String::new()));
                    s.set_values(field, values);
                })
        }))
    }
    fn render_key(&self) -> DiffKey {
        (&self.field).into()
    }
}
#[derive(PartialEq)]
struct TextValue {
    field: FrameKey,
    index: usize,
    mixed: bool,
    removable: bool,
}
impl Component for TextValue {
    fn render(&self) -> impl IntoElement {
        let mut session = use_consume::<State<Session>>();
        let (field, index) = (self.field, self.index);
        let s = session.read().clone();
        let initial = s
            .values(field)
            .get(index)
            .map(text_value)
            .unwrap_or_default();
        let mut value = use_state(|| initial.clone());
        if *value.peek() != initial {
            value.set(initial);
        }
        rect()
            .horizontal()
            .spacing(8.)
            .content(Content::Flex)
            .child(
                Input::new(value)
                    .enabled(!s.busy)
                    .width(Size::flex(1.))
                    .placeholder(if self.mixed {
                        "Mixed values — editing applies to all tracks"
                    } else {
                        "Not set"
                    })
                    .on_validate(move |v: InputValidator| {
                        if *v.text() == *value.peek() {
                            return;
                        }
                        let mut s = session.write();
                        let mut values = s.values(field);
                        if values.len() <= index {
                            values.resize_with(index + 1, || TagValue::Text(String::new()));
                        }
                        match &mut values[index] {
                            TagValue::Comment { text, .. } => *text = v.text().clone(),
                            other => *other = TagValue::Text(v.text().clone()),
                        }
                        s.set_values(field, values);
                    }),
            )
            .maybe_child(self.removable.then(|| {
                Button::new()
                    .compact()
                    .enabled(!s.busy)
                    .child("Remove")
                    .on_press(move |_| {
                        let mut s = session.write();
                        let mut values = s.values(field);
                        if index < values.len() {
                            values.remove(index);
                        }
                        s.set_values(field, values);
                    })
            }))
    }
    fn render_key(&self) -> DiffKey {
        (&(self.field, self.index)).into()
    }
}
