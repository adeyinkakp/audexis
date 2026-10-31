use crate::{
    services::AppServices,
    ui::{
        hooks::{use_library, use_playback},
        theme::use_audexis_colors,
    },
};
use freya::{icons::lucide, prelude::*};
use std::sync::Arc;

#[derive(PartialEq)]
pub(crate) struct FavoriteButton {
    pub id: i64,
}
impl Component for FavoriteButton {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let library = use_library();
        let colors = use_audexis_colors();
        let id = self.id;
        let source = use_reactive(&(id, library.revision()));
        let query_services = services.clone();
        let query_runtime = runtime.clone();
        let mut loader = use_hook(move || {
            FutureTask::create(move || {
                let (id, _) = *source.read();
                let db = query_services.db.clone();
                let runtime = query_runtime.clone();
                async move {
                    let result = runtime.spawn(async move {
                    sqlx::query_scalar::<_, bool>("SELECT COALESCE(m.loved,0) FROM files f LEFT JOIN media_info m ON m.file_id=f.id WHERE f.id=?")
                        .bind(id).fetch_optional(&db.pool).await.map_err(|e| e.to_string())
                }).await.map_err(|e| e.to_string()).and_then(|r| r);
                    (id, result)
                }
            })
        });
        use_side_effect_with_deps(&(id, library.revision()), move |_| {
            loader.cancel();
            loader.start();
        });
        let result = loader.state();
        let loved = result
            .try_as_fulfilled()
            .filter(|(loaded, _)| *loaded == id)
            .and_then(|(_, r)| r.as_ref().ok())
            .copied()
            .flatten();
        let mut busy = use_state(|| false);
        let mut error = use_playback().error;
        let title = if loved == Some(true) {
            "Remove from Favorites"
        } else {
            "Add to Favorites"
        };
        TooltipContainer::new(Tooltip::new(title)).child(
            Button::new()
                .compact()
                .flat()
                .enabled(id > 0 && loved.is_some() && !*busy.read())
                .child(
                    SvgViewer::new(lucide::heart())
                        .width(Size::px(18.))
                        .height(Size::px(18.))
                        .fill(if loved == Some(true) {
                            colors.primary
                        } else {
                            colors.transparent
                        })
                        .color(if loved == Some(true) {
                            colors.primary
                        } else {
                            colors.muted_foreground
                        })
                        .a11y_alt(title),
                )
                .on_press(move |event: Event<PressEventData>| {
                    event.stop_propagation();
                    if *busy.peek() {
                        return;
                    }
                    busy.set(true);
                    let services = services.clone();
                    let runtime = runtime.clone();
                    let library = library.clone();
                    spawn(async move {
                        let db = services.db.clone();
                        let result = runtime
                            .spawn(async move {
                                db.favorite_selection(&[id], !loved.unwrap_or(false))
                                    .await
                                    .map_err(|e| e.to_string())?;
                                services.library_files.publish_changed(vec![id]);
                                Ok::<_, String>(())
                            })
                            .await
                            .map_err(|e| e.to_string())
                            .and_then(|r| r);
                        match result {
                            Ok(()) => {
                                library.refresh();
                                loader.cancel();
                                loader.start();
                                error.set(None);
                            }
                            Err(message) => {
                                error.set(Some(format!("Could not update Favorites: {message}")))
                            }
                        }
                        busy.set(false);
                    });
                }),
        )
    }
}
