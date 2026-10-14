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
        let status = use_library().watcher_status();
        let colors = use_audexis_colors();
        let mut choosing = use_state(|| false);
        let mut error = use_state(|| None::<String>);
        let watcher = services.watcher.clone();
        let rescan = watcher.clone();
        rect()
            .spacing(8.0)
            .child(
                rect()
                    .horizontal()
                    .spacing(10.0)
                    .child(
                        Button::new()
                            .filled()
                            .enabled(watcher.is_some() && !*choosing.read())
                            .child(if *choosing.read() {
                                "Adding folder…"
                            } else {
                                "Add music folder"
                            })
                            .on_press(move |_| {
                                let watcher = watcher.clone();
                                let runtime = runtime.clone();
                                choosing.set(true);
                                error.set(None);
                                spawn(async move {
                                    if let (Some(watcher), Some(folder)) =
                                        (watcher, rfd::AsyncFileDialog::new().pick_folder().await)
                                    {
                                        let path = folder.path().to_owned();
                                        let result = runtime
                                            .spawn(async move { watcher.add_folder(&path).await })
                                            .await;
                                        error.set(match result {
                                            Ok(Ok(())) => None,
                                            Ok(Err(e)) => Some(e.to_string()),
                                            Err(e) => Some(e.to_string()),
                                        });
                                    }
                                    choosing.set(false);
                                });
                            }),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .enabled(rescan.is_some() && !status.busy)
                            .child("Rescan folders")
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
    }
}
