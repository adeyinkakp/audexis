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
fn custom_part(s: &Session, file: usize, key: FrameKey, index: usize, name: bool) -> &String {
    match &s.files[file].tags[&key][index] {
        TagValue::UserText(v) => {
            if name {
                &v.description
            } else {
                &v.value
            }
        }
        TagValue::UserUrl(v) => {
            if name {
                &v.description
            } else {
                &v.url
            }
        }
        _ => unreachable!(),
    }
}
fn custom_part_mut(
    s: &mut Session,
    file: usize,
    key: FrameKey,
    index: usize,
    name: bool,
) -> &mut String {
    match &mut s.files[file].tags.get_mut(&key).unwrap()[index] {
        TagValue::UserText(v) => {
            if name {
                &mut v.description
            } else {
                &mut v.value
            }
        }
        TagValue::UserUrl(v) => {
            if name {
                &mut v.description
            } else {
                &mut v.url
            }
        }
        _ => unreachable!(),
    }
}
impl Component for CustomRow {
    fn render(&self) -> impl IntoElement {
        let mut session = use_consume::<State<Session>>();
        let (file, key, index) = (self.file, self.key, self.index);
        let busy = session.read().busy;
        let colors = crate::ui::theme::use_audexis_colors();
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
                                                        let name =
                                                            custom_part(&s, file, key, index, true)
                                                                .clone();
                                                        let value = custom_part(
                                                            &s, file, key, index, false,
                                                        )
                                                        .clone();
                                                        s.files[file]
                                                            .tags
                                                            .get_mut(&key)
                                                            .unwrap()
                                                            .remove(index);
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
                                Input::new(session.into_writable().map(
                                    move |s| custom_part(s, file, key, index, true),
                                    move |s| custom_part_mut(s, file, key, index, true),
                                ))
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
                                session.write().files[file]
                                    .tags
                                    .get_mut(&key)
                                    .unwrap()
                                    .remove(index);
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
                value: session.into_writable().map(
                    move |s| custom_part(s, file, key, index, false),
                    move |s| custom_part_mut(s, file, key, index, false),
                ),
                enabled: !busy,
                height: 88.,
                on_change: EventHandler::new(|_: String| {}),
            })
    }

    fn render_key(&self) -> DiffKey {
        (&(self.file, self.key, self.index)).into()
    }
}
