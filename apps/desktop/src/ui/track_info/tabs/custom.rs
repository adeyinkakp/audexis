use super::super::*;

#[derive(PartialEq)]
pub struct Custom;
impl Component for Custom {
    fn render(&self) -> impl IntoElement {
        let mut session = use_consume::<State<Session>>();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<Handle>();
        let s = session.read().clone();
        let selected = s.selected;
        let supported = audexis_core::tag_manager::custom_fields::supported(
            &s.files[selected].original.tag_format,
        );
        let colors = crate::ui::theme::use_audexis_colors();
        let mut list = rect()
            .width(Size::fill())
            .spacing(16.)
            .child(
                label()
                    .font_size(12.)
                    .color(colors.muted_foreground)
                    .text("Custom fields are edited per track. Standard fields belong in Details."),
            )
            .maybe_child((s.files.len() > 1).then(|| {
                Select::new()
                    .selected_item(
                        s.files[selected]
                            .original
                            .path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                    )
                    .children(s.files.iter().enumerate().map(|(index, file)| {
                        MenuItem::new()
                            .selected(index == selected)
                            .child(
                                file.original
                                    .path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                            )
                            .on_press(move |_| {
                                if !session.peek().busy {
                                    session.write().selected = index;
                                }
                            })
                            .into_element()
                    }))
            }));
        if !supported {
            return list
                .child(label().text("This format does not support custom fields."))
                .into_element();
        }
        for key in [FrameKey::UserDefinedText, FrameKey::UserDefinedURL] {
            if let Some(values) = s.files[selected].tags.get(&key) {
                for index in 0..values.len() {
                    list = list.child(CustomRow {
                        file: selected,
                        key,
                        index,
                    });
                }
            }
            list = list.child(
                Button::new()
                    .enabled(!s.busy)
                    .child(if key == FrameKey::UserDefinedText {
                        "Add text field"
                    } else {
                        "Add URL field"
                    })
                    .on_press(move |_| {
                        let value = if key == FrameKey::UserDefinedText {
                            TagValue::UserText(UserTextEntry {
                                description: String::new(),
                                value: String::new(),
                            })
                        } else {
                            TagValue::UserUrl(UserUrlEntry {
                                description: String::new(),
                                url: String::new(),
                            })
                        };
                        session.write().files[selected]
                            .tags
                            .entry(key)
                            .or_default()
                            .push(value);
                    }),
            );
        }
        let mut saved = Select::new().selected_item("Choose a saved field…");
        if let Ok(config) = services.user_config.lock() {
            for def in &config.custom_field_definitions {
                let key = if def.kind == audexis_core::config::CustomFieldKind::Url {
                    FrameKey::UserDefinedURL
                } else {
                    FrameKey::UserDefinedText
                };
                let name = def.key.clone();
                saved = saved.child(MenuItem::new().child(def.title.clone()).on_press(move |_| {
                    if session.peek().busy {
                        return;
                    }
                    let value = if key == FrameKey::UserDefinedText {
                        TagValue::UserText(UserTextEntry {
                            description: name.clone(),
                            value: String::new(),
                        })
                    } else {
                        TagValue::UserUrl(UserUrlEntry {
                            description: name.clone(),
                            url: String::new(),
                        })
                    };
                    session.write().files[selected]
                        .tags
                        .entry(key)
                        .or_default()
                        .push(value);
                }));
            }
        }
        list = list
            .child(label().text("Add a saved field").font_size(14.))
            .child(saved)
            .child(
                Button::new()
                    .outline()
                    .child("Manage saved fields…")
                    .on_press(move |_| {
                        crate::ui::launch_settings(services.clone(), runtime.clone());
                    }),
            );
        list.into_element()
    }
}
#[derive(PartialEq)]
struct CustomRow {
    file: usize,
    key: FrameKey,
    index: usize,
}
fn custom_part(
    s: &Session,
    file: usize,
    key: FrameKey,
    index: usize,
    name: bool,
) -> Option<&String> {
    match s.files.get(file)?.tags.get(&key)?.get(index)? {
        TagValue::UserText(v) => {
            if name {
                Some(&v.description)
            } else {
                Some(&v.value)
            }
        }
        TagValue::UserUrl(v) => {
            if name {
                Some(&v.description)
            } else {
                Some(&v.url)
            }
        }
        _ => None,
    }
}
fn custom_part_mut(
    s: &mut Session,
    file: usize,
    key: FrameKey,
    index: usize,
    name: bool,
) -> Option<&mut String> {
    match s.files.get_mut(file)?.tags.get_mut(&key)?.get_mut(index)? {
        TagValue::UserText(v) => {
            if name {
                Some(&mut v.description)
            } else {
                Some(&mut v.value)
            }
        }
        TagValue::UserUrl(v) => {
            if name {
                Some(&mut v.description)
            } else {
                Some(&mut v.url)
            }
        }
        _ => None,
    }
}
impl Component for CustomRow {
    fn render(&self) -> impl IntoElement {
        let mut session = use_consume::<State<Session>>();
        let (file, key, index) = (self.file, self.key, self.index);
        let snapshot = session.read();
        let busy = snapshot.busy;
        let current_name = custom_part(&snapshot, file, key, index, true).cloned();
        let current_value = custom_part(&snapshot, file, key, index, false).cloned();
        let mut name = use_state(|| current_name.clone().unwrap_or_default());
        let mut value = use_state(|| current_value.clone().unwrap_or_default());
        name.set_if_modified(current_name.clone().unwrap_or_default());
        value.set_if_modified(current_value.clone().unwrap_or_default());
        drop(snapshot);
        let colors = crate::ui::theme::use_audexis_colors();
        if current_name.is_none() || current_value.is_none() {
            return label()
                .text("This custom field has an unsupported value type. Its existing value will be preserved.")
                .color(colors.muted_foreground)
                .into_element();
        }
        rect()
            .width(Size::fill())
            .padding(16.)
            .spacing(12.)
            .corner_radius(12.)
            .border(Border::new().fill(colors.border).width(1.))
            .child(label().text(format!("Field {}", index + 1)).font_size(14.))
            .child(
                rect()
                    .horizontal()
                    .content(Content::Flex)
                    .spacing(12.)
                    .child(
                        rect()
                            .width(Size::px(112.))
                            .spacing(6.)
                            .child(
                                label()
                                    .text("Type")
                                    .font_size(12.)
                                    .color(colors.muted_foreground),
                            )
                            .child(
                                Select::new()
                                    .selected_item(if key == FrameKey::UserDefinedText {
                                        "Text"
                                    } else {
                                        "URL"
                                    })
                                    .children(
                                        [FrameKey::UserDefinedText, FrameKey::UserDefinedURL]
                                            .into_iter()
                                            .map(move |kind| {
                                                MenuItem::new()
                                                    .selected(kind == key)
                                                    .child(if kind == FrameKey::UserDefinedText {
                                                        "Text"
                                                    } else {
                                                        "URL"
                                                    })
                                                    .on_press(move |_| {
                                                        if busy || kind == key {
                                                            return;
                                                        }
                                                        let mut s = session.write();
                                                        if s.busy {
                                                            return;
                                                        }
                                                        let Some(name) =
                                                            custom_part(&s, file, key, index, true)
                                                                .cloned()
                                                        else {
                                                            return;
                                                        };
                                                        let Some(value) = custom_part(
                                                            &s, file, key, index, false,
                                                        )
                                                        .cloned() else {
                                                            return;
                                                        };
                                                        let Some(values) =
                                                            s.files.get_mut(file).and_then(
                                                                |file| file.tags.get_mut(&key),
                                                            )
                                                        else {
                                                            return;
                                                        };
                                                        if index >= values.len() {
                                                            return;
                                                        }
                                                        values.remove(index);
                                                        let entry =
                                                            if kind == FrameKey::UserDefinedText {
                                                                TagValue::UserText(UserTextEntry {
                                                                    description: name,
                                                                    value,
                                                                })
                                                            } else {
                                                                TagValue::UserUrl(UserUrlEntry {
                                                                    description: name,
                                                                    url: value,
                                                                })
                                                            };
                                                        s.files[file]
                                                            .tags
                                                            .entry(kind)
                                                            .or_default()
                                                            .push(entry);
                                                    })
                                                    .into_element()
                                            }),
                                    ),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .spacing(6.)
                            .child(
                                label()
                                    .text("Metadata key")
                                    .font_size(12.)
                                    .color(colors.muted_foreground),
                            )
                            .child(
                                Input::new(name)
                                    .on_validate(move |input: InputValidator| {
                                        let mut state = session.write();
                                        if state.busy {
                                            return;
                                        }
                                        if let Some(part) =
                                            custom_part_mut(&mut state, file, key, index, true)
                                        {
                                            *part = input.text().clone();
                                        }
                                    })
                                    .width(Size::fill())
                                    .enabled(!busy)
                                    .placeholder("Field name"),
                            ),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .enabled(!busy)
                            .child("Remove")
                            .on_press(move |_| {
                                let mut state = session.write();
                                if state.busy {
                                    return;
                                }
                                if let Some(values) = state
                                    .files
                                    .get_mut(file)
                                    .and_then(|file| file.tags.get_mut(&key))
                                    && index < values.len()
                                {
                                    values.remove(index);
                                }
                            }),
                    ),
            )
            .child(
                label()
                    .text("Value for this song")
                    .font_size(12.)
                    .color(colors.muted_foreground),
            )
            .child(crate::ui::track_info::text_area::TextArea {
                value: value.into(),
                enabled: !busy,
                height: 88.,
                on_change: EventHandler::new(move |text: String| {
                    let mut state = session.write();
                    if state.busy {
                        return;
                    }
                    if let Some(part) = custom_part_mut(&mut state, file, key, index, false) {
                        *part = text;
                    }
                }),
            })
            .into_element()
    }

    fn render_key(&self) -> DiffKey {
        (&(self.file, self.key, self.index)).into()
    }
}
