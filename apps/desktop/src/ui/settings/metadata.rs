use crate::{services::AppServices, ui::theme::use_audexis_colors};
use audexis_core::tag_manager::field_catalog;
use freya::prelude::*;
use std::sync::Arc;
pub struct MetadataPanel {
    pub fields: State<Vec<String>>,
    pub services: Arc<AppServices>,
}
impl PartialEq for MetadataPanel {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Component for MetadataPanel {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let search = use_state(String::new);
        let mut error = use_state(|| None::<String>);
        let query = search.read().to_lowercase();
        let mut fields = field_catalog::catalog(None)
            .into_iter()
            .filter(|f| {
                f.editable
                    && matches!(f.kind, "text" | "url")
                    && format!("{} {}", f.label, f.storage_key)
                        .to_lowercase()
                        .contains(&query)
            })
            .collect::<Vec<_>>();
        fields.sort_by_key(|f| f.label.to_lowercase());
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .content(Content::Flex)
            .spacing(16.)
            .child(
                rect()
                    .spacing(8.)
                    .child(
                        label()
                            .text("Details fields")
                            .font_size(20.)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        label()
                            .text("Choose which metadata fields you edit.")
                            .color(colors.muted_foreground),
                    ),
            )
            .child(
                label()
                    .text("Choose fields to show in Track Info’s Details tab.")
                    .font_size(14.)
                    .color(colors.muted_foreground),
            )
            .child(
                Input::new(search)
                    .width(Size::fill())
                    .placeholder("Find a field…"),
            )
            .maybe_child(
                error
                    .read()
                    .clone()
                    .map(|e| label().text(e).font_size(12.).color(colors.destructive)),
            )
            .child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .spacing(4.)
                    .children(fields.into_iter().map(|field| {
                        let mut selected = self.fields;
                        let services = self.services.clone();
                        let key = field.storage_key.clone();
                        let checked = selected.read().contains(&key);
                        Button::new()
                            .flat()
                            .width(Size::fill())
                            .child(
                                rect()
                                    .horizontal()
                                    .width(Size::fill())
                                    .spacing(12.)
                                    .cross_align(Alignment::Center)
                                    .child(Checkbox::new().selected(checked))
                                    .child(label().text(field.label).font_size(14.)),
                            )
                            .on_press(move |_| {
                                let mut values = selected.peek().clone();
                                if values.contains(&key) {
                                    values.retain(|v| v != &key);
                                } else {
                                    values.push(key.clone());
                                }
                                match services.update_config(|c| c.metadata_fields = values.clone())
                                {
                                    Ok(()) => {
                                        selected.set(values);
                                        error.set(None);
                                    }
                                    Err(e) => error.set(Some(e.to_string())),
                                }
                            })
                            .into_element()
                    })),
            )
    }
}
