use crate::{
    services::AppServices,
    ui::{routes::Route, theme::use_audexis_colors},
};
use freya::{prelude::*, router::RouterContext};
use std::sync::Arc;

#[derive(PartialEq)]
pub(super) struct RewindPlaylist {
    pub year: i32,
    pub month: Option<u32>,
    pub available: bool,
}

impl Component for RewindPlaylist {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let router = RouterContext::get();
        let colors = use_audexis_colors();
        let mut busy = use_state(|| false);
        let mut cloud = use_state(|| false);
        let mut error = use_state(|| None::<String>);
        let mut created = use_state(|| None::<(i32, Option<u32>, i64)>);
        let year = self.year;
        let month = self.month;
        rect()
            .width(Size::fill())
            .spacing(8.)
            .child(
                rect()
                    .horizontal()
                    .spacing(12.)
                    .child(
                        Button::new()
                            .enabled(!*busy.read())
                            .flat()
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .child(Checkbox::new().selected(*cloud.read()))
                                    .child("Sync playlist to cloud"),
                            )
                            .on_press(move |_| {
                                let value = *cloud.peek();
                                cloud.set(!value);
                            }),
                    )
                    .child(
                        Button::new()
                            .filled()
                            .enabled(self.available && !*busy.read())
                            .child(if *busy.read() {
                                "Creating…"
                            } else {
                                "Create playlist for this period"
                            })
                            .on_press(move |_| {
                                busy.set(true);
                                error.set(None);
                                let services = services.clone();
                                let runtime = runtime.clone();
                                let sync = *cloud.peek();
                                spawn(async move {
                                    let db = services.db.clone();
                                    let result = runtime
                                        .spawn(async move {
                                            db.create_rewind_playlist(year, month, sync).await
                                        })
                                        .await
                                        .map_err(|error| error.to_string())
                                        .and_then(|result| result);
                                    busy.set(false);
                                    match result {
                                        Ok((id, _)) => {
                                            created.set(Some((year, month, id)));
                                            services.library_files.publish_changed(vec![]);
                                            services.sync_edits();
                                        }
                                        Err(message) => error.set(Some(message)),
                                    }
                                });
                            }),
                    )
                    .maybe_child(
                        created
                            .read()
                            .filter(|(y, m, _)| *y == year && *m == month)
                            .map(|(_, _, id)| {
                                Button::new()
                                    .outline()
                                    .child("Open playlist")
                                    .on_press(move |_| {
                                        let _ = router.push(Route::Playlist { playlist_id: id });
                                    })
                            }),
                    ),
            )
            .maybe_child(
                error
                    .read()
                    .clone()
                    .map(|message| label().text(message).color(colors.destructive)),
            )
    }
}
