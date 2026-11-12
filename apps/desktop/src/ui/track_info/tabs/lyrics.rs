use super::super::*;
use audexis_core::metadata_editor::{format_lrc, parse_lrc};
#[derive(PartialEq)]
pub struct Lyrics {
    pub synced: bool,
}
impl Component for Lyrics {
    fn render(&self) -> impl IntoElement {
        let mut session = use_consume::<State<Session>>();
        let key = if self.synced {
            FrameKey::SynchronizedLyrics
        } else {
            FrameKey::UnsyncedLyrics
        };
        let s = session.read().clone();
        let initial = s.text(key);
        let mut value = use_state(|| initial.clone());
        if *value.peek() != initial {
            value.set(initial.clone());
        }
        let supported = s
            .files
            .iter()
            .all(|f| audexis_core::tag_manager::custom_fields::supported(&f.original.tag_format));
        let enabled = !s.busy && supported;
        rect()
            .width(Size::fill())
            .spacing(16.)
            .child(
                label()
                    .text(if self.synced { "Synced lyrics" } else { "Lyrics" })
                    .font_size(14.),
            )
            .child(
                label()
                    .font_size(12.)
                    .color(crate::ui::theme::use_audexis_colors().muted_foreground)
                    .text(if self.synced {
                        concat!(
                            "Edit line timing (press Enter to apply a timestamp), preview with audio, ",
                            "or edit Raw LRC.",
                        )
                    } else {
                        "Edit embedded lyrics. Line breaks are preserved. Clear and save to remove lyrics."
                    }),
            )
            .maybe_child(
                (!supported).then(|| label().text("One or more selected file formats do not support lyrics editing.")),
            )
            .maybe_child((s.mixed(key) || s.values(key).len() > 1).then(|| {
                label().text(concat!(
                    "Showing the first entry. Saving edits replaces this lyrics type in all ",
                    "selected tracks.",
                ))
            }))
            .child(crate::ui::track_info::lookup::Lookup {
                kind: if self.synced { Tab::Synced } else { Tab::Plain },
            })
            .child(if self.synced {
                Synced {
                    value: value.into_writable(),
                    enabled,
                }
                .into_element()
            } else {
                crate::ui::track_info::text_area::TextArea {
                    height: 360.,
                    value: value.into_writable(),
                    enabled,
                    on_change: EventHandler::new(move |text: String| {
                        session.write().set_values(
                            key,
                            if text.is_empty() {
                                vec![]
                            } else {
                                vec![TagValue::Text(text)]
                            },
                        );
                    }),
                }
                .into_element()
            })
            .child(
                label()
                    .font_size(12.)
                    .color(crate::ui::theme::use_audexis_colors().muted_foreground)
                    .text(s.files[0].original.path.display().to_string()),
            )
    }
    fn render_key(&self) -> DiffKey {
        (&self.synced).into()
    }
}
#[derive(PartialEq)]
struct Synced {
    value: Writable<String>,
    enabled: bool,
}
impl Component for Synced {
    fn render(&self) -> impl IntoElement {
        let mut session = use_consume::<State<Session>>();
        let mut mode = use_state(|| 0usize);
        let mut editing = use_state(|| None::<usize>);
        let colors = crate::ui::theme::use_audexis_colors();
        let status = crate::ui::track_info::preview::use_preview(
            session.read().files[0].original.path.display().to_string(),
        );
        let position = status.status.read().position_ms;
        let text = self.value.read().clone();
        let parsed = parse_lrc(&text);
        let enabled = self.enabled;
        let mut body = rect()
            .width(Size::fill())
            .spacing(20.)
            .padding(24.)
            .min_height(Size::px(440.))
            .corner_radius(16.)
            .background(colors.surface_subtle)
            .child(rect().horizontal().spacing(8.).children(
                ["Edit", "Preview", "Raw LRC"].into_iter().enumerate().map(
                    move |(index, title)| {
                        Button::new()
                            .flat()
                            .compact()
                            .theme_colors(ButtonColorsThemePartial {
                                background: Some(
                                    (if *mode.read() == index {
                                        colors.control_fill
                                    } else {
                                        colors.surface_subtle
                                    })
                                    .into(),
                                ),
                                ..Default::default()
                            })
                            .child(title)
                            .on_press(move |_| mode.set(index))
                            .into_element()
                    },
                ),
            ));
        if *mode.read() == 2 {
            body = body.child(crate::ui::track_info::text_area::TextArea {
                height: 360.,
                value: self.value.clone(),
                enabled,
                on_change: EventHandler::new(move |text: String| {
                    session.write().set_values(
                        FrameKey::SynchronizedLyrics,
                        if text.is_empty() {
                            vec![]
                        } else {
                            vec![TagValue::Text(text)]
                        },
                    )
                }),
            });
        } else if let Ok(rows) = &parsed {
            let active = rows
                .iter()
                .rposition(|(time, _)| u64::from(*time) <= position);
            for (index, (time, text)) in rows.iter().enumerate() {
                if *mode.read() == 1 || *editing.read() != Some(index) {
                    let preview = status.clone();
                    let time = *time;
                    let preview_mode = *mode.read() == 1;
                    body = body.child(
                        Button::new()
                            .flat()
                            .child(
                                label()
                                    .text(if text.is_empty() {
                                        "New line".to_string()
                                    } else {
                                        text.clone()
                                    })
                                    .font_size(28.)
                                    .color(if active == Some(index) {
                                        colors.foreground
                                    } else {
                                        colors.muted_foreground
                                    }),
                            )
                            .on_press(move |_| {
                                if preview_mode {
                                    preview.command(
                                        audexis_audio::lyrics_preview::Action::Seek,
                                        u64::from(time),
                                    );
                                } else {
                                    editing.set(Some(index));
                                }
                            }),
                    );
                } else {
                    body = body.child(LyricRow {
                        index,
                        time: *time,
                        text: text.clone(),
                        position,
                        enabled,
                    });
                }
            }
            body = body.child(
                Button::new()
                    .enabled(enabled)
                    .child("Add line at current time")
                    .on_press(move |_| {
                        let mut rows =
                            parse_lrc(&session.peek().text(FrameKey::SynchronizedLyrics))
                                .unwrap_or_default();
                        rows.push((position.min(u64::from(u32::MAX)) as u32, String::new()));
                        commit_rows(session, rows);
                    }),
            );
        }
        body.child(crate::ui::track_info::preview::Controls { preview: status })
            .maybe_child(
                parsed.err().map(|error| {
                    label().text(format!("Invalid LRC: {error}. Use Raw LRC to fix it."))
                }),
            )
    }
}
fn commit_rows(mut session: State<Session>, mut rows: Vec<(u32, String)>) {
    rows.sort_by_key(|row| row.0);
    session.write().set_values(
        FrameKey::SynchronizedLyrics,
        vec![TagValue::Text(format_lrc(&rows))],
    );
}
#[derive(PartialEq)]
struct LyricRow {
    index: usize,
    time: u32,
    text: String,
    position: u64,
    enabled: bool,
}
impl Component for LyricRow {
    fn render(&self) -> impl IntoElement {
        let session = use_consume::<State<Session>>();
        let index = self.index;
        let mut text = use_state(|| self.text.clone());
        if *text.peek() != self.text {
            text.set(self.text.clone());
        }
        let initial = format!(
            "{:02}:{:02}.{:03}",
            self.time / 60000,
            self.time / 1000 % 60,
            self.time % 1000
        );
        let mut stamp = use_state(|| initial.clone());
        use_side_effect_with_deps(&self.time, move |_| stamp.set(initial.clone()));
        let position = self.position;
        rect()
            .horizontal()
            .spacing(6.)
            .content(Content::Flex)
            .child(
                Input::new(stamp)
                    .enabled(self.enabled)
                    .width(Size::px(135.))
                    .on_submit(move |value: String| {
                        if let Ok(parsed) = parse_lrc(&format!("[{value}]")) {
                            let mut rows =
                                parse_lrc(&session.peek().text(FrameKey::SynchronizedLyrics))
                                    .unwrap_or_default();
                            if index < rows.len() {
                                rows[index].0 = parsed[0].0;
                                commit_rows(session, rows);
                            }
                        }
                    }),
            )
            .child(
                Input::new(text)
                    .enabled(self.enabled)
                    .width(Size::flex(1.))
                    .on_validate(move |value: InputValidator| {
                        let mut rows =
                            parse_lrc(&session.peek().text(FrameKey::SynchronizedLyrics))
                                .unwrap_or_default();
                        if index < rows.len() {
                            rows[index].1 = value.text().clone();
                            commit_rows(session, rows);
                        }
                    }),
            )
            .children([-250i64, 250, 0, i64::MAX].into_iter().map(move |offset| {
                Button::new()
                    .enabled(self.enabled)
                    .child(match offset {
                        -250 => "−250ms",
                        250 => "+250ms",
                        0 => "Now",
                        _ => "Remove",
                    })
                    .on_press(move |_| {
                        let mut rows =
                            parse_lrc(&session.peek().text(FrameKey::SynchronizedLyrics))
                                .unwrap_or_default();
                        if index >= rows.len() {
                            return;
                        }
                        if offset == i64::MAX {
                            rows.remove(index);
                        } else {
                            rows[index].0 = if offset == 0 {
                                position.min(u64::from(u32::MAX)) as u32
                            } else {
                                (i64::from(rows[index].0) + offset).clamp(0, i64::from(u32::MAX))
                                    as u32
                            };
                        }
                        commit_rows(session, rows);
                    })
                    .into_element()
            }))
    }
}
