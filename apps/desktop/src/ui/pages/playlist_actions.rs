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
        if *editing.read() {
            return rect()
                .width(Size::fill())
                .padding(20.)
                .spacing(18.)
                .corner_radius(12.)
                .background(colors.surface_subtle)
                .child(
                    label()
                        .text("Edit playlist")
                        .font_size(20.)
                        .font_weight(FontWeight::BOLD),
                )
                .child(crate::ui::components::PlaylistForm {
                    playlist_id: Some(id),
                    name,
                    on_saved: EventHandler::new(move |(_, title)| {
                        editing.set(false);
                        changed.call(Some(title));
                    }),
                    on_cancel: Some(EventHandler::new(move |()| editing.set(false))),
                })
                .into_element();
        }
        rect()
            .width(Size::fill())
            .spacing(10.)
            .child(
                rect()
                    .horizontal()
                    .spacing(8.)
                    .child(
                        Button::new()
                            .outline()
                            .enabled(!*busy.read())
                            .child("Edit playlist")
                            .on_press(move |_| {
                                error.set(None);
                                editing.set(true);
                            }),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .enabled(!*busy.read())
                            .child("Delete playlist")
                            .on_press(move |_| {
                                let services = services.clone();
                                let runtime = runtime.clone();
                                let changed = changed.clone();
                                busy.set(true);
                                error.set(None);
                                spawn(async move {
                                    if rfd::AsyncMessageDialog::new()
                                        .set_title("Delete playlist?")
                                        .set_description("Your audio files stay in your library.")
                                        .set_buttons(rfd::MessageButtons::YesNo)
                                        .show()
                                        .await
                                        == rfd::MessageDialogResult::Yes
                                    {
                                        let db = services.db.clone();
                                        match runtime
                                            .spawn(async move {
                                                db.delete_collection_playlist(id)
                                                    .await
                                                    .map_err(|e| e.to_string())
                                            })
                                            .await
                                            .map_err(|e| e.to_string())
                                            .and_then(|r| r)
                                        {
                                            Ok(()) => {
                                                services.library_files.publish_changed(vec![]);
                                                services.sync_edits();
                                                changed.call(None);
                                            }
                                            Err(e) => error.set(Some(e)),
                                        }
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
            )
            .into_element()
    }
    fn render_key(&self) -> DiffKey {
        (&("playlist-actions", self.id)).into()
    }
}
