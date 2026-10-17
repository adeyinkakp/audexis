use audexis_core::tag_manager::field_catalog;
use freya::prelude::*;
use std::sync::Arc;

use crate::services::AppServices;
use crate::ui::theme::use_audexis_colors;

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
        let fields = field_catalog::catalog(None)
            .into_iter()
            .filter(|field| field.editable && field.kind == "text")
            .collect::<Vec<_>>();
        rect()
            .spacing(14.0)
            .child(
                label()
                    .text("Details fields")
                    .font_size(25.0)
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                label()
                    .text("Choose which metadata fields appear in Track Info.")
                    .color(colors.muted_foreground),
            )
            .child(
                ScrollView::new()
                    .height(Size::flex(1.0))
                    .spacing(4.0)
                    .children(fields.into_iter().map(|field| {
                        let mut selected = self.fields;
                        let services = self.services.clone();
                        let key = field.storage_key.clone();
                        let checked = selected.read().contains(&key);
                        Button::new()
                            .flat()
                            .width(Size::fill())
                            .child(format!(
                                "{} {}",
                                if checked { "[x]" } else { "[ ]" },
                                field.label
                            ))
                            .on_press(move |_| {
                                let mut values = selected.write();
                                if values.contains(&key) {
                                    values.retain(|value| value != &key);
                                } else {
                                    values.push(key.clone());
                                }
                                let saved = values.clone();
                                let _ =
                                    services.update_config(|config| config.metadata_fields = saved);
                            })
                            .into_element()
                    })),
            )
            .child(
                label()
                    .text("Field selections are saved automatically.")
                    .font_size(12.0)
                    .color(colors.muted_foreground),
            )
    }
}
