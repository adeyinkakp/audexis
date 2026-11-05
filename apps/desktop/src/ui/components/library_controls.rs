use crate::{
    services::AppServices,
    ui::{hooks::use_library, theme::use_audexis_colors},
};
use freya::prelude::*;
use std::sync::Arc;

#[derive(PartialEq)]
pub struct LibraryControls;
impl Component for LibraryControls {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let library = use_library();
        let status = library.watcher_status();
        use_side_effect_with_deps(&status.busy, move |busy| {
            if !busy {
                library.refresh();
            }
        });
        let colors = use_audexis_colors();
        let mut choosing = use_state(|| false);
        let mut error = use_state(|| None::<String>);
        let watcher = services.watcher.clone();
        let rescan = watcher.clone();
        rect().width(Size::fill()).spacing(8.0).child(
            rect()
                .width(Size::fill())
                .content(Content::Flex)
                .horizontal()
                .spacing(10.0)
                .maybe_child(
                    (!status.message.is_empty())
                        .then(|| label().text(status.message).color(colors.muted_foreground)),
                )
                .maybe_child(
                    status
                        .error
                        .or_else(|| error.read().clone())
                        .map(|e| label().text(e).color(colors.destructive)),
                )
                .child(rect().width(Size::flex(1.)))
                .child(
                    Button::new()
                        .outline()
                        .enabled(rescan.is_some() && !status.busy)
                        .child(if status.busy {
                            "Refreshing…"
                        } else {
                            "Refresh Library"
                        })
                        .on_press(move |_| {
                            error.set(
                                rescan
                                    .as_ref()
                                    .and_then(|w| w.rescan().err())
                                    .map(|e| e.to_string()),
                            );
                        }),
                ),
        )
    }
}
