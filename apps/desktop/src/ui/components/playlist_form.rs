use crate::{services::AppServices, ui::theme::use_audexis_colors};
use freya::prelude::*;
use std::sync::Arc;

#[derive(PartialEq)]
pub(crate) struct PlaylistForm {
    pub name: State<String>,
    pub on_created: EventHandler<(i64, String)>,
    pub on_cancel: Option<EventHandler<()>>,
}
impl Component for PlaylistForm {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let colors = use_audexis_colors();
        let name = self.name;
        let mut busy = use_state(|| false);
        let mut error = use_state(|| None::<String>);
        let created = self.on_created.clone();
        rect()
            .width(Size::fill())
            .spacing(12.)
            .child(
                Input::new(name)
                    .width(Size::fill())
                    .placeholder("Playlist name")
                    .enabled(!*busy.read()),
            )
            .child(
                rect()
                    .horizontal()
                    .spacing(8.)
                    .child(
                        Button::new()
                            .filled()
                            .child("Create")
                            .enabled(!*busy.read() && !name.read().trim().is_empty())
                            .on_press(move |_| {
                                if *busy.peek() {
                                    return;
                                }
                                busy.set(true);
                                error.set(None);
                                let services = services.clone();
                                let runtime = runtime.clone();
                                let created = created.clone();
                                let title = name.peek().trim().to_string();
                                spawn(async move {
                                    let db = services.db.clone();
                                    let requested = title.clone();
                                    let result = runtime
                                        .spawn(async move {
                                            db.create_collection_playlist(&requested).await
                                        })
                                        .await
                                        .map_err(|e| e.to_string())
                                        .and_then(|r| r);
                                    busy.set(false);
                                    match result {
                                        Ok(id) => {
                                            services.library_files.publish_changed(vec![]);
                                            created.call((id, title));
                                        }
                                        Err(e) => error
                                            .set(Some(format!("Could not create playlist: {e}"))),
                                    }
                                });
                            }),
                    )
                    .maybe_child(self.on_cancel.clone().map(|cancel| {
                        Button::new()
                            .enabled(!*busy.read())
                            .child("Cancel")
                            .on_press(move |_| cancel.call(()))
                    })),
            )
            .maybe_child(
                error
                    .read()
                    .clone()
                    .map(|e| label().text(e).color(colors.destructive)),
            )
    }
}
