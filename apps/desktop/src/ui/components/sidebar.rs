use crate::ui::{
    hooks::use_library,
    routes::Route,
    theme::{AudexisColors, use_audexis_colors},
};
use freya::{icons::lucide, prelude::*};
use std::sync::Arc;
#[derive(PartialEq)]
pub struct Sidebar;
impl Component for Sidebar {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<crate::services::AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let revision = use_library().revision();
        let db = services.db.clone();
        let rt = runtime.clone();
        let mut retry = use_state(|| 0u64);
        let mut loader = use_hook(move || {
            FutureTask::create(move || {
                let db = db.clone();
                let rt = rt.clone();
                async move {
                    rt.spawn(async move {
                        sqlx::query_as::<_, (i64, String)>(
                            "SELECT id,name FROM library_playlists ORDER BY name COLLATE NOCASE,id",
                        )
                        .fetch_all(&db.pool)
                        .await
                        .map_err(|e| e.to_string())
                    })
                    .await
                    .map_err(|e| e.to_string())?
                }
            })
        });
        use_side_effect_with_deps(&(revision, *retry.read()), move |_| {
            loader.cancel();
            loader.start();
        });
        let state = loader.state();
        let mut retained = use_state(|| None::<Result<Vec<(i64, String)>, String>>);
        let result = state.try_as_fulfilled().cloned();
        use_side_effect_with_deps(&result, move |result| {
            if let Some(Ok(items)) = result {
                retained.set_if_modified(Some(Ok(items.clone())));
            }
        });
        let previous = retained.read();
        let mut playlists = ScrollView::new()
            .width(Size::fill())
            .height(Size::flex(1.))
            .spacing(4.);
        match state.try_as_fulfilled().or(previous.as_ref()) {
            Some(Ok(items)) => {
                for (id, title) in items {
                    playlists = playlists.child(nav(
                        Route::Playlist { playlist_id: *id },
                        title,
                        lucide::list_music(),
                        &colors,
                    ));
                }
            }
            Some(Err(_)) => {
                playlists = playlists.child(
                    Button::new()
                        .flat()
                        .child("Retry playlists")
                        .on_press(move |_| *retry.write() += 1),
                )
            }
            None => {
                playlists = playlists.child(
                    label()
                        .text("Loading playlists…")
                        .font_size(12.)
                        .color(colors.muted_foreground),
                )
            }
        }
        rect()
            .width(Size::px(220.))
            .height(Size::fill())
            .content(Content::Flex)
            .padding((12., 12., 12., 12.))
            .spacing(4.)
            .background(colors.background)
            .child(nav(Route::Home, "Home", lucide::house(), &colors))
            .child(nav(Route::Rewind, "Rewind", lucide::history(), &colors))
            .child(nav(Route::Search, "Search", lucide::search(), &colors))
            .child(section("LIBRARY", &colors))
            .child(nav(
                Route::CloudLibrary,
                "Cloud Sync",
                lucide::cloud(),
                &colors,
            ))
            .child(nav(Route::Songs, "Songs", lucide::music(), &colors))
            .child(nav(Route::Artists, "Artists", lucide::users(), &colors))
            .child(nav(Route::Albums, "Albums", lucide::disc(), &colors))
            .child(nav(Route::Favorites, "Favorites", lucide::heart(), &colors))
            .child(nav(
                Route::Missing,
                "Missing Files",
                lucide::folder_search(),
                &colors,
            ))
            .child(section("PLAYLISTS", &colors))
            .child(nav(
                Route::Playlists,
                "All Playlists",
                lucide::list_music(),
                &colors,
            ))
            .child(playlists)
            .child(super::sync_indicator::SyncIndicator)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(1.))
                    .background(colors.border_subtle)
                    .margin((8., 0.)),
            )
            .child(
                SideBarItem::new()
                    .padding((6., 10.))
                    .color(colors.foreground)
                    .background(colors.background)
                    .hover_background(colors.hover)
                    .focus_border_fill(colors.ring)
                    .child(item("Settings", lucide::settings_2(), &colors))
                    .on_press(move |_| {
                        crate::ui::launch_settings(services.clone(), runtime.clone())
                    }),
            )
    }
}
fn section(title: &str, colors: &AudexisColors) -> impl IntoElement {
    label()
        .text(crate::ui::display_text(title.to_owned()))
        .font_size(11.)
        .color(colors.muted_foreground)
        .margin((8., 12., 4., 12.))
}
fn item(title: &str, icon: Bytes, colors: &AudexisColors) -> impl IntoElement {
    rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::Flex)
        .spacing(10.)
        .cross_align(Alignment::Center)
        .child(
            SvgViewer::new(icon)
                .width(Size::px(18.))
                .height(Size::px(18.))
                .stroke(colors.muted_foreground),
        )
        .child(
            label()
                .text(crate::ui::display_text(title.to_owned()))
                .width(Size::flex(1.))
                .font_size(14.)
                .max_lines(1)
                .text_overflow(TextOverflow::Ellipsis),
        )
}
fn nav(route: Route, title: &str, icon: Bytes, colors: &AudexisColors) -> impl IntoElement {
    ActivableRoute::new(
        route.clone(),
        Link::new(route).child(
            SideBarItem::new()
                .padding((6., 10.))
                .color(colors.foreground)
                .background(colors.background)
                .hover_background(colors.hover)
                .active_background(colors.primary_washed)
                .focus_border_fill(colors.ring)
                .child(item(title, icon, colors)),
        ),
    )
    .exact(true)
}
