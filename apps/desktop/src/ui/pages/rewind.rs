use crate::{
    services::AppServices,
    ui::{components::SmoothScroll, hooks::use_library, theme::use_audexis_colors},
};
use freya::prelude::*;
use std::sync::Arc;
#[derive(PartialEq)]
pub struct Rewind;
impl Component for Rewind {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let library = use_library();
        let mut year = use_state(|| None::<i32>);
        let mut month = use_state(|| None::<u32>);
        let mut retry = use_state(|| 0u64);
        let key = (
            *year.read(),
            *month.read(),
            library.revision(),
            *retry.read(),
        );
        let source = use_reactive(&key);
        let mut loader = use_hook(move || {
            FutureTask::create(move || {
                let db = services.db.clone();
                let runtime = runtime.clone();
                let (year, month, _, _) = *source.peek();
                async move {
                    runtime
                        .spawn(async move {
                            let year = match year {
                                Some(y) => y,
                                None => sqlx::query_scalar::<_, i32>(
                                    "SELECT CAST(strftime('%Y','now','localtime') AS INTEGER)",
                                )
                                .fetch_one(&db.pool)
                                .await
                                .map_err(|e| e.to_string())?,
                            };
                            let data =
                                audexis_core::database::rewind::read(&db.pool, year, month).await?;
                            let songs = db
                                .songs_by_ids(
                                    &data.songs.iter().map(|s| s.file_id).collect::<Vec<_>>(),
                                )
                                .await
                                .map_err(|e| e.to_string())?;
                            Ok::<_, String>((year, data, songs))
                        })
                        .await
                        .map_err(|e| e.to_string())?
                }
            })
        });
        use_side_effect_with_deps(&key, move |_| {
            loader.cancel();
            loader.start();
        });
        let state = loader.state();
        let result = state.try_as_fulfilled();
        let mut body = rect()
            .width(Size::fill())
            .padding(28.)
            .spacing(24.)
            .child(
                label()
                    .text("YOUR LISTENING HISTORY")
                    .font_size(12.)
                    .color(colors.primary),
            )
            .child(
                label()
                    .text("Rewind")
                    .font_size(36.)
                    .font_weight(FontWeight::BOLD),
            );
        if let Some(Ok((current, data, songs))) = result {
            let mut years = data.years.clone();
            if !years.contains(&i64::from(*current)) {
                years.push(i64::from(*current));
                years.sort_by(|a, b| b.cmp(a));
            }
            body = body.child(Select::new().selected_item(current.to_string()).children(
                years.into_iter().map(move |y| {
                    MenuItem::new()
                        .selected(y == i64::from(*current))
                        .child(y.to_string())
                        .on_press(move |_| {
                            year.set(Some(y as i32));
                            month.set(None);
                        })
                        .into_element()
                }),
            ));
            body = body.child(
                ScrollView::new()
                    .direction(Direction::Horizontal)
                    .height(Size::px(48.))
                    .child(
                        rect().horizontal().spacing(8.).children(
                            [
                                "Full Year",
                                "Jan",
                                "Feb",
                                "Mar",
                                "Apr",
                                "May",
                                "Jun",
                                "Jul",
                                "Aug",
                                "Sep",
                                "Oct",
                                "Nov",
                                "Dec",
                            ]
                            .into_iter()
                            .enumerate()
                            .map(move |(i, title)| {
                                let choice = if i == 0 { None } else { Some(i as u32) };
                                Button::new()
                                    .flat()
                                    .theme_colors(ButtonColorsThemePartial {
                                        background: Some(
                                            (if *month.read() == choice {
                                                colors.primary_washed
                                            } else {
                                                colors.surface_subtle
                                            })
                                            .into(),
                                        ),
                                        ..Default::default()
                                    })
                                    .child(title)
                                    .on_press(move |_| month.set(choice))
                                    .into_element()
                            }),
                        ),
                    ),
            );
            body = body.child(
                rect()
                    .width(Size::fill())
                    .padding(28.)
                    .corner_radius(24.)
                    .spacing(12.)
                    .background(colors.primary_washed)
                    .child(
                        label()
                            .text(format!(
                                "{} · {current}",
                                (*month.read())
                                    .map(|m| format!("Month {m}"))
                                    .unwrap_or_else(|| "Full year".into())
                            ))
                            .color(colors.muted_foreground),
                    )
                    .child(
                        label()
                            .text(minutes(data.total_us))
                            .font_size(32.)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        label()
                            .text("Actual listening time.")
                            .font_size(12.)
                            .color(colors.muted_foreground),
                    ),
            );
            if data.total_us == 0 {
                body = body
                    .child(
                        label()
                            .text("Your listening time starts here")
                            .font_size(22.),
                    )
                    .child(
                        label()
                            .text("No listening time recorded for this period.")
                            .color(colors.muted_foreground),
                    );
            }
            for (title, cards) in [
                (
                    "Top Artists",
                    data.artists
                        .iter()
                        .map(|s| (s.file_id, s.name.clone(), minutes(s.listened_us)))
                        .collect::<Vec<_>>(),
                ),
                (
                    "Top Songs",
                    data.songs
                        .iter()
                        .map(|s| {
                            (
                                s.file_id,
                                songs
                                    .iter()
                                    .find(|t| t.id == s.file_id)
                                    .map(|s| s.title.clone())
                                    .unwrap_or_default(),
                                minutes(s.listened_us),
                            )
                        })
                        .collect(),
                ),
                (
                    "Top Albums",
                    data.albums
                        .iter()
                        .map(|s| {
                            (
                                s.file_id,
                                s.name.clone(),
                                format!("{} · {}", s.artist, minutes(s.listened_us)),
                            )
                        })
                        .collect(),
                ),
                (
                    "Top Playlists",
                    data.playlists
                        .iter()
                        .map(|s| (s.file_id, s.name.clone(), minutes(s.listened_us)))
                        .collect(),
                ),
            ] {
                if cards.is_empty() {
                    continue;
                }
                if title == "Top Songs" {
                    let tracks = data
                        .songs
                        .iter()
                        .enumerate()
                        .filter_map(|(index, entry)| {
                            songs
                                .iter()
                                .find(|song| song.id == entry.file_id)
                                .map(|song| {
                                    let mut track = song.clone();
                                    track.title = format!("{}. {}", index + 1, song.title);
                                    track.artist = minutes(entry.listened_us);
                                    track
                                })
                        })
                        .collect();
                    body = body.child(crate::ui::components::DiscoveryShelf {
                        title: "Top Songs",
                        subtitle: "Your most played tracks",
                        empty: "No tracks",
                        tracks,
                    });
                    continue;
                }
                body = body
                    .child(
                        label()
                            .text(title)
                            .font_size(24.)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        ScrollView::new()
                            .direction(Direction::Horizontal)
                            .height(Size::px(245.))
                            .child(
                                rect().horizontal().spacing(24.).children(
                                    cards.into_iter().enumerate().map(
                                        |(i, (id, title, subtitle))| {
                                            crate::ui::components::ArtworkCard {
                                                file_id: id,
                                                title: format!("{}. {title}", i + 1),
                                                subtitle,
                                                size: 170.,
                                                radius: 0.,
                                            }
                                            .into_element()
                                        },
                                    ),
                                ),
                            ),
                    );
            }
        } else if let Some(Err(e)) = result {
            body = body.child(Button::new().child(format!("{e} · Try again")).on_press(
                move |_| {
                    *retry.write() += 1;
                },
            ));
        } else {
            body = body.child(label().text("Loading your listening time…"));
        }
        SmoothScroll::new().child(body.child(rect().height(Size::px(80.))))
    }
}
fn minutes(us: i64) -> String {
    format!("{} minutes", us / 60_000_000)
}
