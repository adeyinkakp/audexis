use crate::ui::theme::use_audexis_colors;
use audexis_core::cloud_sync::protocol::{SmartField, SmartPlaylist, SmartRule, SmartSort};
use freya::prelude::*;

#[derive(PartialEq)]
pub(crate) struct PlaylistOptions {
    pub smart: State<bool>,
    pub rules: State<SmartPlaylist>,
    pub cloud: State<bool>,
    pub busy: bool,
}
impl Component for PlaylistOptions {
    fn render(&self) -> impl IntoElement {
        let mut smart = self.smart;
        let mut cloud = self.cloud;
        let mut rules = self.rules;
        let busy = self.busy;
        let colors = use_audexis_colors();
        let current = rules.read().clone();
        let history = *smart.read() && current.uses_listening_history();
        use_side_effect_with_deps(&history, move |history| {
            if *history {
                cloud.set_if_modified(false);
            }
        });
        let limit = use_state(|| current.limit.map(|n| n.to_string()).unwrap_or_default());
        let limit_value = limit.read().clone();
        use_side_effect_with_deps(&limit_value, move |value| {
            let parsed = if value.trim().is_empty() {
                None
            } else {
                Some(value.trim().parse::<u32>().unwrap_or(0))
            };
            if rules.peek().limit != parsed {
                rules.write().limit = parsed;
            }
        });
        let mut types = rect().horizontal().spacing(8.);
        for (value, title) in [(false, "Standard"), (true, "Smart")] {
            let button = Button::new()
                .enabled(!busy)
                .child(title)
                .on_press(move |_| {
                    smart.set_if_modified(value);
                });
            types = types.child(if *smart.read() == value {
                button.filled()
            } else {
                button.outline()
            });
        }
        let mut locations = rect().horizontal().spacing(8.);
        for (value, title) in [(false, "This computer"), (true, "Cloud")] {
            let button = Button::new()
                .enabled(!busy && (!value || !history))
                .child(title)
                .on_press(move |_| {
                    cloud.set_if_modified(value);
                });
            locations = locations.child(if *cloud.read() == value {
                button.filled()
            } else {
                button.outline()
            });
        }
        let mut body = rect()
            .width(Size::fill())
            .spacing(18.)
            .child(
                rect()
                    .spacing(8.)
                    .child(label().text("Playlist type").font_size(13.))
                    .child(types),
            )
            .child(
                rect()
                    .spacing(8.)
                    .child(label().text("Sync").font_size(13.))
                    .child(locations)
                    .child(label().font_size(12.).color(colors.muted_foreground).text(
                        if history && *smart.read() {
                            "Listening-history smart playlists stay on this computer because listening history is stored per device."
                        } else if *cloud.read() {
                            if *smart.read() {
                                "Matches your cloud library and syncs across devices."
                            } else {
                                "Syncs with your account. A local copy stays on this computer."
                            }
                        } else if *smart.read() {
                            "Matches music on this computer."
                        } else {
                            "Saved on this computer only."
                        },
                    )),
            );
        if !*smart.read() {
            return body.into_element();
        }
        body = body.child(
            rect()
                .width(Size::fill())
                .horizontal()
                .spacing(8.)
                .cross_align(Alignment::Center)
                .child(label().text("Match").font_size(13.))
                .child(
                    Select::new()
                        .selected_item(if current.match_all {
                            "all rules"
                        } else {
                            "any rule"
                        })
                        .children([(true, "all rules"), (false, "any rule")].into_iter().map(
                            move |(all, text)| {
                                MenuItem::new()
                                    .selected(current.match_all == all)
                                    .child(text)
                                    .on_press(move |_| {
                                        if !busy {
                                            rules.write().match_all = all;
                                        }
                                    })
                                    .into_element()
                            },
                        )),
                ),
        );
        for (index, rule) in current.rules.iter().enumerate() {
            let field = rule.field;
            let operator = rule.operator;
            let field_select = Select::new().selected_item(field.label()).children(
                SmartField::ALL.into_iter().map(move |value| {
                    MenuItem::new()
                        .selected(value == field)
                        .child(value.label())
                        .on_press(move |_| {
                            if busy {
                                return;
                            }
                            let mut draft = rules.write();
                            let rule = &mut draft.rules[index];
                            if rule.field == value {
                                return;
                            }
                            rule.field = value;
                            rule.operator = value.operators()[0];
                            rule.value = match value {
                                SmartField::Favorite => "true",
                                SmartField::AddedDays => "30",
                                SmartField::PlayCount | SmartField::ListeningMinutes => "0",
                                _ => "",
                            }
                            .into();
                        })
                        .into_element()
                }),
            );
            let comparison = Select::new().selected_item(operator.label()).children(
                field.operators().iter().copied().map(move |value| {
                    MenuItem::new()
                        .selected(value == operator)
                        .child(value.label())
                        .on_press(move |_| {
                            if !busy {
                                rules.write().rules[index].operator = value;
                            }
                        })
                        .into_element()
                }),
            );
            let value: Element = if field == SmartField::Favorite {
                Select::new()
                    .selected_item(if rule.value == "true" { "Yes" } else { "No" })
                    .children([("true", "Yes"), ("false", "No")].into_iter().map(
                        move |(value, title)| {
                            MenuItem::new()
                                .child(title)
                                .on_press(move |_| {
                                    if !busy {
                                        rules.write().rules[index].value = value.into();
                                    }
                                })
                                .into_element()
                        },
                    ))
                    .into_element()
            } else {
                Input::new(rules.into_writable().map(
                    move |r| &r.rules[index].value,
                    move |r| &mut r.rules[index].value,
                ))
                .width(Size::fill())
                .enabled(!busy)
                .placeholder("Value")
                .into_element()
            };
            body = body.child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::Flex)
                    .spacing(8.)
                    .cross_align(Alignment::Center)
                    .child(field_select)
                    .child(comparison)
                    .child(rect().width(Size::flex(1.)).child(value))
                    .child(
                        Button::new()
                            .flat()
                            .enabled(!busy && current.rules.len() > 1)
                            .child("Remove")
                            .on_press(move |_| {
                                rules.write().rules.remove(index);
                            }),
                    ),
            );
        }
        body = body
            .child(
                Button::new()
                    .outline()
                    .enabled(!busy && current.rules.len() < 20)
                    .child("Add rule")
                    .on_press(move |_| rules.write().rules.push(SmartRule::default())),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .spacing(24.)
                    .cross_align(Alignment::End)
                    .child(
                        rect()
                            .spacing(8.)
                            .child(label().text("Sort by").font_size(13.))
                            .child(Select::new().selected_item(current.sort.label()).children(
                                SmartSort::ALL.into_iter().map(move |sort| {
                                    MenuItem::new()
                                        .selected(sort == current.sort)
                                        .child(sort.label())
                                        .on_press(move |_| {
                                            if !busy {
                                                rules.write().sort = sort;
                                            }
                                        })
                                        .into_element()
                                }),
                            )),
                    )
                    .child(
                        rect()
                            .spacing(8.)
                            .child(label().text("Maximum songs").font_size(13.))
                            .child(
                                Input::new(limit)
                                    .enabled(!busy)
                                    .placeholder("10,000")
                                    .width(Size::px(140.)),
                            ),
                    ),
            );
        if history {
            body = body.child(super::smart_playlist_period::SmartPlaylistPeriod { rules, busy });
        }
        body.child(
            Button::new()
                .flat()
                .enabled(!busy)
                .child(
                    rect()
                        .horizontal()
                        .spacing(8.)
                        .child(Checkbox::new().selected(current.live_updating))
                        .child("Live updating"),
                )
                .on_press(move |_| {
                    let enabled = rules.peek().live_updating;
                    rules.write().live_updating = !enabled;
                }),
        )
        .child(
            label()
                .font_size(12.)
                .color(colors.muted_foreground)
                .text("When turned off results refresh only when you save this playlist."),
        )
        .into_element()
    }
}
