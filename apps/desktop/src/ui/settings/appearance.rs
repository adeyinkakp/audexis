use audexis_core::config::{Density, ThemePreference};
use freya::prelude::*;

use crate::services::AppServices;
use crate::ui::theme::{ThemeController, use_audexis_colors};
use std::sync::Arc;

pub struct AppearancePanel {
    pub theme: ThemeController,
}

impl PartialEq for AppearancePanel {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Component for AppearancePanel {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let dark = self.theme.preference() == ThemePreference::Dark;
        let mut controller = self.theme.clone();
        let mut error = use_state(|| None::<String>);
        let services = use_consume::<Arc<AppServices>>();
        let config = services
            .user_config
            .lock()
            .map(|config| (config.density, config.reduce_motion))
            .unwrap_or((Density::Default, false));
        let density = use_state(|| config.0);
        let mut reduce_motion = use_state(|| config.1);

        rect()
            .spacing(18.0)
            .child(
                label()
                    .text("Make yourself at home")
                    .font_size(25.0)
                    .font_weight(FontWeight::BOLD),
            )
            .child(label().text("Density").font_weight(FontWeight::BOLD))
            .child(
                rect().horizontal().spacing(8.0).children(
                    [Density::Default, Density::Compact, Density::Comfort]
                        .into_iter()
                        .map(|value| {
                            let mut density = density;
                            let services = services.clone();
                            Button::new()
                                .flat()
                                .child(format!("{value:?}"))
                                .on_press(move |_| {
                                    density.set(value);
                                    let _ = services.update_config(|config| config.density = value);
                                })
                                .into_element()
                        }),
                ),
            )
            .child(
                Button::new()
                    .flat()
                    .child(if *reduce_motion.read() {
                        "Enable motion"
                    } else {
                        "Reduce motion"
                    })
                    .on_press(move |_| {
                        reduce_motion.toggle();
                        let value = *reduce_motion.read();
                        let _ = services.update_config(|config| config.reduce_motion = value);
                    }),
            )
            .child(
                label()
                    .text("Change how Audexis looks.")
                    .color(colors.muted_foreground),
            )
            .child(
                rect()
                    .padding(18.0)
                    .spacing(12.0)
                    .background(colors.card)
                    .corner_radius(12.0)
                    .border(Border::new().width(1.0).fill(colors.border_subtle))
                    .child(label().text("Theme").font_weight(FontWeight::BOLD))
                    .child(
                        Button::new()
                            .filled()
                            .child(if dark {
                                "Use light theme"
                            } else {
                                "Use dark theme"
                            })
                            .on_press(move |_| {
                                let next = if controller.preference() == ThemePreference::Dark {
                                    ThemePreference::Light
                                } else {
                                    ThemePreference::Dark
                                };
                                error.set(controller.set(next).err().map(|e| e.to_string()));
                            }),
                    )
                    .maybe_child(
                        error
                            .read()
                            .clone()
                            .map(|e| label().text(e).color(colors.destructive)),
                    ),
            )
    }
}
