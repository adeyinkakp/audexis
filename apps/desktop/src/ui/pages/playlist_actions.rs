use crate::{services::AppServices, ui::theme::use_audexis_colors};
use freya::prelude::*;
use std::sync::Arc;
#[derive(PartialEq)]
pub struct PlaylistActions {
    pub id: i64,
    pub name: String,
    pub on_change: EventHandler<Option<String>>,
}
impl Component for PlaylistActions {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let colors = use_audexis_colors();
        let mut editing = use_state(|| false);
        let name = use_state(|| self.name.clone());
        let mut busy = use_state(|| false);
        let mut error = use_state(|| None::<String>);
        let id = self.id;
        let changed = self.on_change.clone();
        let mut body = rect().width(Size::fill()).spacing(12.).child(
            Button::new()
                .outline()
                .child(if *editing.read() {
                    "Done editing"
                } else {
                    "Edit playlist"
                })
                .on_press(move |_| editing.toggle()),
        );
        if !*editing.read() {
            return body.into_element();
        }
        let db = services.db.clone();
        let rt = runtime.clone();
        let callback = changed.clone();
        body = body
            .padding(20.)
            .corner_radius(16.)
            .background(colors.surface_subtle)
            .child(
                label()
                    .text("Playlist name")
                    .font_size(12.)
                    .color(colors.muted_foreground),
            )
            .child(
                rect()
                    .horizontal()
                    .spacing(8.)
                    .child(
                        Input::new(name)
                            .enabled(!*busy.read())
                            .width(Size::px(300.)),
                    )
                    .child(
                        Button::new()
                            .filled()
                            .enabled(!*busy.read())
                            .child("Save name")
                            .on_press(move |_| {
                                let db = db.clone();
                                let rt = rt.clone();
                                let callback = callback.clone();
                                let value = name.peek().clone();
                                busy.set(true);
                                spawn(async move {
                                    let requested = value.clone();
                                    match rt
                                        .spawn(async move {
                                            db.rename_collection_playlist(id, &requested).await
                                        })
                                        .await
                                        .map_err(|e| e.to_string())
                                        .and_then(|r| r)
                                    {
                                        Ok(()) => callback.call(Some(value)),
                                        Err(e) => error.set(Some(e)),
                                    }
                                    busy.set(false);
                                });
                            }),
                    ),
            )
            .maybe_child(
                error
                    .read()
                    .clone()
                    .map(|e| label().text(e).color(colors.destructive)),
            );
        let db = services.db.clone();
        let rt = runtime.clone();
        body.child(
            Button::new()
                .flat()
                .enabled(!*busy.read())
                .child("Delete playlist")
                .on_press(move |_| {
                    let db = db.clone();
                    let rt = rt.clone();
                    let callback = changed.clone();
                    busy.set(true);
                    spawn(async move {
                        if rfd::AsyncMessageDialog::new()
                            .set_title("Delete playlist?")
                            .set_description("Your audio files stay in your library.")
                            .set_buttons(rfd::MessageButtons::YesNo)
                            .show()
                            .await
                            == rfd::MessageDialogResult::Yes
                        {
                            match rt
                                .spawn(async move {
                                    db.delete_collection_playlist(id)
                                        .await
                                        .map_err(|e| e.to_string())
                                })
                                .await
                                .map_err(|e| e.to_string())
                                .and_then(|r| r)
                            {
                                Ok(()) => callback.call(None),
                                Err(e) => error.set(Some(e)),
                            }
                        }
                        busy.set(false);
                    });
                }),
        )
        .into_element()
    }
    fn render_key(&self) -> DiffKey {
        (&("playlist-actions", self.id)).into()
    }
}
