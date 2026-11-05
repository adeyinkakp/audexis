use super::*;

#[derive(PartialEq)]
pub(super) struct Review {
    pub kind: Tab,
    pub item: Option<Value>,
    pub chosen: State<Vec<FrameKey>>,
}
impl Component for Review {
    fn render(&self) -> impl IntoElement {
        let mut session = use_consume::<State<Session>>();
        let runtime = use_consume::<Handle>();
        let colors = crate::ui::theme::use_audexis_colors();
        let mut chosen = self.chosen;
        let mut message = use_state(String::new);
        use_side_effect_with_deps(&self.item, move |_| message.set(String::new()));
        let busy = session.read().busy;
        let mut body = rect()
            .width(Size::fill())
            .spacing(12.)
            .padding(16.)
            .corner_radius(12.)
            .background(colors.background);
        let Some(item) = &self.item else {
            return body
                .min_height(Size::px(350.))
                .child(label().text("Preview").font_size(16.))
                .child(
                    label()
                        .text("Select a result to review what will be added.")
                        .font_size(12.)
                        .color(colors.muted_foreground),
                )
                .into_element();
        };
        let fields = fields(self.kind, item);
        body = body
            .child(label().text(title(item)).font_size(16.).max_lines(2))
            .child(
                label()
                    .text(subtitle(self.kind, item))
                    .font_size(12.)
                    .max_lines(2)
                    .color(colors.muted_foreground),
            );
        if self.kind == Tab::Details {
            let all: Vec<_> = fields.iter().map(|field| field.key).collect();
            body = body.child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .spacing(6.)
                    .child(label().text("Fields to apply").font_size(12.))
                    .child(rect().width(Size::flex(1.)))
                    .child(
                        Button::new()
                            .compact()
                            .flat()
                            .enabled(!busy)
                            .child("All")
                            .on_press(move |_| chosen.set(all.clone())),
                    )
                    .child(
                        Button::new()
                            .compact()
                            .flat()
                            .enabled(!busy)
                            .child("None")
                            .on_press(move |_| chosen.set(Vec::new())),
                    ),
            );
            body = body.child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::px(260.))
                    .spacing(4.)
                    .children(fields.iter().map(|field| {
                        let key = field.key;
                        let checked = chosen.read().contains(&key);
                        Button::new()
                            .flat()
                            .width(Size::fill())
                            .enabled(!busy)
                            .child(
                                rect()
                                    .horizontal()
                                    .width(Size::fill())
                                    .content(Content::Flex)
                                    .cross_align(Alignment::Center)
                                    .spacing(10.)
                                    .child(Checkbox::new().selected(checked))
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .spacing(3.)
                                            .child(
                                                label()
                                                    .text(field.title)
                                                    .font_size(11.)
                                                    .color(colors.muted_foreground),
                                            )
                                            .child(
                                                label()
                                                    .text(if key == FrameKey::AttachedPicture {
                                                        "Replace front cover".to_owned()
                                                    } else {
                                                        field.value.clone()
                                                    })
                                                    .max_lines(2)
                                                    .font_size(13.),
                                            ),
                                    ),
                            )
                            .on_press(move |_| {
                                let mut keys = chosen.write();
                                if keys.contains(&key) {
                                    keys.retain(|value| *value != key);
                                } else {
                                    keys.push(key);
                                }
                            })
                            .into_element()
                    })),
            );
        } else {
            body = body.child(
                label()
                    .text(if self.kind == Tab::Synced {
                        "Synced lyrics"
                    } else {
                        "Plain lyrics"
                    })
                    .font_size(12.)
                    .color(colors.muted_foreground),
            );
            body = body.child(ScrollView::new().width(Size::fill()).height(Size::px(260.))
                .child(label().text(fields.first().map(|field| field.value.clone()).unwrap_or_else(|| {
                    if self.kind == Tab::Synced {
                        "No synced lyrics on this match. Choose a result marked Synced lyrics.".into()
                    } else {
                        "No plain lyrics on this match. Choose a result marked Plain lyrics.".into()
                    }
                })).font_size(13.)));
        }
        let selected: Vec<_> = fields
            .into_iter()
            .filter(|field| chosen.read().contains(&field.key))
            .collect();
        let count = selected.len();
        let files = session.read().files.len();
        body = body.child(
            label()
                .text(if files > 1 {
                    format!("Applies to all {files} selected tracks. Other fields stay unchanged.")
                } else {
                    "Only the selected fields will change.".into()
                })
                .font_size(11.)
                .color(colors.muted_foreground),
        );
        body.child(Button::new().filled().width(Size::fill()).enabled(!busy && count > 0)
            .child(if busy { "Applying…".into() } else if self.kind == Tab::Details {
                format!("Apply {count} selected {}", if count == 1 { "field" } else { "fields" })
            } else { "Use these lyrics".into() })
            .on_press(move |_| {
                if session.peek().busy { return; }
                let runtime = runtime.clone();
                let selected = selected.clone();
                session.write().busy = true;
                message.set(String::new());
                spawn(async move {
                    let prepared = runtime.spawn(fields::prepare(selected)).await;
                    let mut s = session.write();
                    match prepared {
                        Ok(Ok(changes)) => {
                            for (key, value) in changes {
                                if key == FrameKey::AttachedPicture {
                                    for file in &mut s.files {
                                        let images = file.tags.entry(key).or_default();
                                        images.retain(|value| !matches!(value, TagValue::Picture { picture_type: Some(3), .. }));
                                        images.insert(0, value.clone());
                                    }
                                } else {
                                    s.set_values(key, vec![value]);
                                }
                            }
                            s.revision += 1;
                            message.set("Added to the editor. Save the affected tabs to keep your changes.".into());
                        }
                        Ok(Err(error)) => message.set(error),
                        Err(_) => message.set("Could not apply this result. Please try again.".into()),
                    }
                    s.busy = false;
                });
            }))
            .maybe_child((!message.read().is_empty()).then(|| label().text(message.read().clone()).font_size(12.)))
            .into_element()
    }
}
