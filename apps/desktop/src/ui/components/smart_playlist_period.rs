use audexis_core::cloud_sync::protocol::{ListeningPeriod, SmartPlaylist};
use freya::prelude::*;
use std::sync::Arc;

#[derive(PartialEq)]
pub(crate) struct SmartPlaylistPeriod {
    pub rules: State<SmartPlaylist>,
    pub busy: bool,
}

impl Component for SmartPlaylistPeriod {
    fn render(&self) -> impl IntoElement {
        let mut rules = self.rules;
        let current = rules.read().listening_period;
        let mut year = use_state(|| {
            current
                .map(|period| period.year.to_string())
                .unwrap_or_default()
        });
        let services = use_consume::<Arc<crate::services::AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        use_future(move || {
            let db = services.db.clone();
            let runtime = runtime.clone();
            async move {
                if let Ok(Ok(current)) = runtime
                    .spawn(async move {
                        sqlx::query_scalar::<_, String>("SELECT strftime('%Y','now','localtime')")
                            .fetch_one(&db.pool)
                            .await
                    })
                    .await
                    && year.peek().is_empty()
                {
                    year.set(current);
                }
            }
        });
        let year_text = year.read().clone();
        use_side_effect_with_deps(&year_text, move |value| {
            if let Some(period) = rules.write().listening_period.as_mut() {
                period.year = value.parse().unwrap_or(0);
            }
        });
        let busy = self.busy;
        let months = [
            "Full year",
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        let mut body = rect()
            .width(Size::fill())
            .spacing(8.)
            .child(label().text("Listening history period").font_size(13.))
            .child(
                Select::new()
                    .selected_item(if current.is_some() {
                        "Selected year or month"
                    } else {
                        "All time"
                    })
                    .children([false, true].into_iter().map(move |selected| {
                        MenuItem::new()
                            .selected(current.is_some() == selected)
                            .child(if selected {
                                "Selected year or month"
                            } else {
                                "All time"
                            })
                            .on_press(move |_| {
                                if !busy {
                                    rules.write().listening_period =
                                        selected.then(|| ListeningPeriod {
                                            year: year.peek().parse().unwrap_or(0),
                                            month: None,
                                        });
                                }
                            })
                            .into_element()
                    })),
            );
        if let Some(period) = current {
            body = body.child(
                rect()
                    .horizontal()
                    .spacing(8.)
                    .child(
                        Input::new(year)
                            .enabled(!busy)
                            .placeholder("Year")
                            .width(Size::px(110.)),
                    )
                    .child(
                        Select::new()
                            .selected_item(
                                months
                                    .get(period.month.unwrap_or(0) as usize)
                                    .copied()
                                    .unwrap_or("Full year"),
                            )
                            .children(months.into_iter().enumerate().map(move |(month, title)| {
                                let selected = (month != 0).then_some(month as u32);
                                MenuItem::new()
                                    .selected(period.month == selected)
                                    .child(title)
                                    .on_press(move |_| {
                                        if !busy
                                            && let Some(period) =
                                                rules.write().listening_period.as_mut()
                                        {
                                            period.month = selected;
                                        }
                                    })
                                    .into_element()
                            })),
                    ),
            );
        }
        body.child(
            label()
                .text("Applies to play count, listening time, and listening-based sorting")
                .font_size(12.),
        )
    }
}
