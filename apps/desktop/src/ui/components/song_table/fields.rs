use super::TableSettings;
use crate::{services::AppServices, ui::theme::use_audexis_colors};
use audexis_core::config::*;
use freya::prelude::*;
use std::sync::Arc;
#[derive(PartialEq)]
pub struct CustomFieldsEditor {
    pub settings: TableSettings,
    pub standalone: bool,
}
impl Component for CustomFieldsEditor {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let mut title = use_state(String::new);
        let mut key = use_state(String::new);
        let mut url = use_state(|| false);
        let mut editing = use_state(|| None::<String>);
        let mut validation = use_state(|| None::<String>);
        let mut settings = self.settings.clone();
        let mut close = settings.editing_fields;
        let fields = settings.definitions.read().clone();
        let mut list = rect().width(Size::fill()).spacing(4.0);
        for field in fields {
            let mut remove = self.settings.clone();
            let services = services.clone();
            let edited = field.clone();
            let id = field.id.clone();
            let column = field.column();
            list = list.child(
                rect()
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .spacing(8.0)
                    .child(
                        label()
                            .text(format!(
                                "{} · {} · {}",
                                field.title,
                                field.key,
                                if field.kind == CustomFieldKind::Url {
                                    "URL"
                                } else {
                                    "Text"
                                }
                            ))
                            .width(Size::flex(1.0))
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(12.0),
                    )
                    .child(Button::new().compact().child("Edit").on_press(move |_| {
                        title.set(edited.title.clone());
                        key.set(edited.key.clone());
                        url.set(edited.kind == CustomFieldKind::Url);
                        editing.set(Some(edited.id.clone()));
                        validation.set(None);
                    }))
                    .child(Button::new().compact().child("Remove").on_press(move |_| {
                        remove.definitions.write().retain(|f| f.id != id);
                        {
                            let mut cfg = remove.table.write();
                            cfg.columns.retain(|c| c.column != column);
                            cfg.hidden_columns.retain(|c| c.column != column);
                            if cfg.sort == column {
                                cfg.sort = SongColumn::Title;
                                cfg.descending = false;
                            }
                            cfg.normalize();
                        }
                        if editing.peek().as_ref() == Some(&id) {
                            editing.set(None);
                            title.set(String::new());
                            key.set(String::new());
                        }
                        remove.save(&services);
                    })),
            );
        }
        rect()
            .width(Size::fill())
            .padding(12.0)
            .spacing(8.0)
            .background(colors.card)
            .corner_radius(8.0)
            .maybe_child((!self.standalone).then(|| {
                rect()
                    .horizontal()
                    .content(Content::Flex)
                    .child(
                        label()
                            .text("Custom text / URL columns")
                            .width(Size::flex(1.0))
                            .font_size(14.0),
                    )
                    .child(
                        Button::new()
                            .compact()
                            .child("Done")
                            .on_press(move |_| close.set(false)),
                    )
            }))
            .child(
                label()
                    .text(concat!(
                        "Map a column title to a metadata key. Values come from your files; this ",
                        "does not edit their tags.",
                    ))
                    .font_size(11.0)
                    .color(colors.muted_foreground),
            )
            .maybe_child((!settings.definitions.read().is_empty()).then(|| {
                ScrollView::new()
                    .height(Size::px(if self.standalone { 220. } else { 100. }))
                    .child(list)
            }))
            .child(
                rect()
                    .horizontal()
                    .content(Content::Flex)
                    .spacing(8.0)
                    .child(
                        Input::new(title)
                            .placeholder("Column title")
                            .width(Size::flex(1.0)),
                    )
                    .child(
                        Input::new(key)
                            .placeholder("Metadata key, e.g. SOURCE")
                            .width(Size::flex(1.0)),
                    )
                    .child(
                        Button::new()
                            .compact()
                            .child(if *url.read() {
                                "Type: URL"
                            } else {
                                "Type: Text"
                            })
                            .on_press(move |_| url.toggle()),
                    )
                    .child(
                        Button::new()
                            .compact()
                            .child(if editing.read().is_some() {
                                "Save field"
                            } else {
                                "Add field"
                            })
                            .on_press(move |_| {
                                let field = CustomFieldDefinition {
                                    id: editing
                                        .peek()
                                        .clone()
                                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                                    title: title.peek().trim().to_string(),
                                    key: key.peek().trim().to_string(),
                                    kind: if *url.peek() {
                                        CustomFieldKind::Url
                                    } else {
                                        CustomFieldKind::Text
                                    },
                                };
                                if let Err(message) = field.validate(&settings.definitions.peek()) {
                                    validation.set(Some(message));
                                    return;
                                }
                                let old = settings
                                    .definitions
                                    .peek()
                                    .iter()
                                    .find(|f| f.id == field.id)
                                    .map(|f| f.column());
                                let new_column = field.column();
                                {
                                    let mut definitions = settings.definitions.write();
                                    if let Some(old) =
                                        definitions.iter_mut().find(|f| f.id == field.id)
                                    {
                                        *old = field;
                                    } else {
                                        definitions.push(field);
                                    }
                                }
                                {
                                    let mut cfg = settings.table.write();
                                    if let Some(old) = old {
                                        for c in &mut cfg.columns {
                                            if c.column == old {
                                                c.column = new_column.clone();
                                            }
                                        }
                                        for c in &mut cfg.hidden_columns {
                                            if c.column == old {
                                                c.column = new_column.clone();
                                            }
                                        }
                                        if cfg.sort == old {
                                            cfg.sort = new_column;
                                        }
                                    } else {
                                        cfg.columns.push(TableColumn::new(new_column));
                                    }
                                    cfg.normalize();
                                }
                                settings.save(&services);
                                validation.set(None);
                                editing.set(None);
                                title.set(String::new());
                                key.set(String::new());
                            }),
                    ),
            )
            .maybe_child(
                validation
                    .read()
                    .clone()
                    .map(|message| label().text(message).color(colors.destructive)),
            )
    }
}
