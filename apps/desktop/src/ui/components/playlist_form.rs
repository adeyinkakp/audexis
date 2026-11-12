use crate::{services::AppServices, ui::theme::use_audexis_colors};
use audexis_core::cloud_sync::protocol::SmartPlaylist;
use freya::prelude::*;
use std::sync::Arc;

#[derive(PartialEq)]
pub(crate) struct PlaylistForm {
    pub playlist_id: Option<i64>,
    pub name: State<String>,
    pub on_saved: EventHandler<(i64, String)>,
    pub on_cancel: Option<EventHandler<()>>,
}
impl Component for PlaylistForm {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let colors = use_audexis_colors();
        let mut name = self.name;
        let id = self.playlist_id;
        let mut loaded = use_state(|| id.is_none());
        let mut busy = use_state(|| false);
        let mut error = use_state(|| None::<String>);
        let saved = self.on_saved.clone();
        let mut smart = use_state(|| false);
        let mut rules = use_state(SmartPlaylist::default);
        let mut cloud = use_state(|| false);
        let db = services.db.clone();
        let rt = runtime.clone();
        use_side_effect_with_deps(&id, move |id| {
            if let Some(id) = *id {
                let db = db.clone();
                let rt = rt.clone();
                spawn(async move {
                    match rt
                        .spawn(async move { db.playlist_settings(id).await })
                        .await
                    {
                        Ok(Ok(settings)) => {
                            name.set(settings.name);
                            smart.set(settings.smart_rules.is_some());
                            cloud.set(settings.cloud_enabled);
                            rules.set(settings.smart_rules.unwrap_or_default());
                            loaded.set(true);
                        }
                        Ok(Err(e)) => error.set(Some(e)),
                        Err(e) => error.set(Some(e.to_string())),
                    }
                });
            }
        });
        let disabled = *busy.read() || !*loaded.read();
        rect()
            .width(Size::fill())
            .spacing(18.)
            .child(
                rect()
                    .width(Size::fill())
                    .spacing(8.)
                    .child(label().text("Name").font_size(13.))
                    .child(
                        Input::new(name)
                            .width(Size::fill())
                            .placeholder("Playlist name")
                            .enabled(!disabled),
                    ),
            )
            .maybe_child(
                (*loaded.read()).then(|| super::smart_playlist_editor::PlaylistOptions {
                    smart,
                    rules,
                    cloud,
                    busy: *busy.read(),
                }),
            )
            .maybe_child((!*loaded.read() && error.read().is_none()).then(|| {
                label()
                    .text("Loading playlist…")
                    .color(colors.muted_foreground)
            }))
            .maybe_child(
                error
                    .read()
                    .clone()
                    .map(|e| label().text(e).color(colors.destructive)),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::Flex)
                    .spacing(8.)
                    .child(rect().width(Size::flex(1.)))
                    .maybe_child(self.on_cancel.clone().map(|cancel| {
                        Button::new()
                            .outline()
                            .enabled(!*busy.read())
                            .child("Cancel")
                            .on_press(move |_| cancel.call(()))
                    }))
                    .child(
                        Button::new()
                            .filled()
                            .child(if *busy.read() {
                                "Saving…"
                            } else if id.is_some() {
                                "Save changes"
                            } else {
                                "Create playlist"
                            })
                            .enabled(!disabled && !name.read().trim().is_empty())
                            .on_press(move |_| {
                                if *busy.peek() {
                                    return;
                                }
                                let spec = (*smart.peek()).then(|| rules.peek().clone());
                                if let Some(spec) = &spec {
                                    if let Err(e) = spec.validate() {
                                        error.set(Some(e));
                                        return;
                                    }
                                }
                                busy.set(true);
                                error.set(None);
                                let services = services.clone();
                                let runtime = runtime.clone();
                                let saved = saved.clone();
                                let title = name.peek().trim().to_string();
                                let sync = *cloud.peek();
                                spawn(async move {
                                    let db = services.db.clone();
                                    let requested = title.clone();
                                    let result = runtime
                                        .spawn(async move {
                                            match id {
                                                Some(id) => {
                                                    db.update_playlist(id, &requested, spec, sync)
                                                        .await?;
                                                    Ok(id)
                                                }
                                                None => {
                                                    db.create_playlist(&requested, spec, sync).await
                                                }
                                            }
                                        })
                                        .await
                                        .map_err(|e| e.to_string())
                                        .and_then(|r| r);
                                    busy.set(false);
                                    match result {
                                        Ok(id) => {
                                            services.library_files.publish_changed(vec![]);
                                            services.sync_edits();
                                            saved.call((id, title));
                                        }
                                        Err(e) => error.set(Some(e)),
                                    }
                                });
                            }),
                    ),
            )
    }
}
