use crate::{
    services::AppServices,
    updater::{self, Phase},
};
use freya::prelude::*;
use std::sync::Arc;
use tokio::runtime::Handle;

#[derive(PartialEq)]
pub struct UpdatesPanel;
impl Component for UpdatesPanel {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<Handle>();
        let mut enabled = use_state(|| {
            services
                .user_config
                .lock()
                .map(|config| config.auto_update_checks)
                .unwrap_or(true)
        });
        let mut error = use_state(|| None::<String>);
        let status = super::super::windows::updates::use_update_status();
        let checking = matches!(status.read().phase, Phase::Checking);
        let save = services.clone();
        rect()
            .width(Size::fill())
            .padding(24.)
            .spacing(20.)
            .child(
                label()
                    .text("Updates")
                    .font_size(22.)
                    .font_weight(FontWeight::BOLD),
            )
            .child(label().text(format!("Audexis {}", env!("CARGO_PKG_VERSION"))))
            .child(
                Button::new()
                    .flat()
                    .width(Size::fill())
                    .child(
                        rect()
                            .horizontal()
                            .spacing(12.)
                            .cross_align(Alignment::Center)
                            .child(Checkbox::new().selected(*enabled.read()))
                            .child(label().text("Automatically check for updates")),
                    )
                    .on_press(move |_| {
                        let value = !*enabled.peek();
                        match save.update_config(|config| config.auto_update_checks = value) {
                            Ok(()) => {
                                enabled.set(value);
                                error.set(None);
                            }
                            Err(message) => error.set(Some(message.to_string())),
                        }
                    }),
            )
            .child(
                label().text(
                    "Check when Audexis starts and every 24 hours. You choose when to install.",
                ),
            )
            .child(
                Button::new()
                    .enabled(!checking)
                    .child(if checking {
                        "Checking…"
                    } else {
                        "Check for updates"
                    })
                    .on_press(move |_| {
                        runtime.spawn(updater::updates().check());
                        super::super::windows::updates::open(services.clone(), runtime.clone());
                    }),
            )
            .maybe_child(
                error
                    .read()
                    .as_ref()
                    .map(|message| label().text(message.clone())),
            )
    }
}
