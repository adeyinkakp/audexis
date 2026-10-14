use crate::ui::{
    components::{DiscoveryShelf, LibraryControls},
    hooks::use_app_state,
    theme::{use_audexis_colors, use_theme_controller},
};
use audexis_core::config::ThemePreference;
use freya::prelude::*;

#[derive(PartialEq)]
pub struct Home;
impl Component for Home {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let library = crate::ui::hooks::use_library();
        let discovery = crate::ui::hooks::use_home_discovery();
        let loading = discovery.loading;
        let result = match discovery.data {
            Some(data) => Some(Ok((*data).clone())),
            None => discovery.error.clone().map(Err),
        };

        let mut controller = use_theme_controller();
        let dark = controller.preference() == ThemePreference::Dark;
        let mut theme_error = use_state(|| None::<String>);
        let mut content = rect()
            .width(Size::fill())
            .padding(28.0)
            .spacing(28.0)
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .spacing(12.0)
                    .child(
                        rect().width(Size::flex(1.0)).spacing(6.0).child(
                            label()
                                .text("Home")
                                .font_size(32.0)
                                .font_weight(FontWeight::BOLD),
                        ),
                    ),
            )
            .child(LibraryControls);
        if let Some(error) = discovery.error {
            content = content.child(label().text(error).color(colors.destructive));
        }
        match result {
            None => {
                content = content.child(
                    label()
                        .text("Finding something to listen to…")
                        .color(colors.muted_foreground),
                )
            }
            Some(Err(error)) => {
                content = content.child(
                    label()
                        .text(format!(
                            "Could not load your music: {error}. Use Refresh to try again."
                        ))
                        .color(colors.destructive),
                )
            }
            Some(Ok(data)) => {
                if data.track_count == 0 {
                    content = content.child(rect().width(Size::fill()).padding(24.0).spacing(10.0).corner_radius(16.0)
                        .background(colors.primary_washed).border(Border::new().width(1.0).fill(colors.primary_border))
                        .child(label().text("Import your own collection").font_size(22.0).font_weight(FontWeight::BOLD))
                        .child(label().text("Your indexed music and listening history will appear here..").color(colors.muted_foreground)));
                } else {
                    content = content.child(
                        label()
                            .text(format!("{} tracks in your collection", data.track_count))
                            .color(colors.muted_foreground),
                    );
                }
                content = content
                    .child(DiscoveryShelf {
                        title: "Recently Played",
                        subtitle: "Pick up where you left off",
                        empty: "Play music from your library and it will appear here.",
                        tracks: data.recent,
                    })
                    .child(DiscoveryShelf {
                        title: "Heavy Rotation",
                        subtitle: "Your most-played songs this month",
                        empty: "Your most-played songs will appear here as you listen.",
                        tracks: data.heavy_rotation,
                    })
                    .child(DiscoveryShelf {
                        title: "Rediscover",
                        subtitle: "Songs you haven’t heard in a while",
                        empty: "Your indexed collection will appear here.",
                        tracks: data.rediscover,
                    })
                    .child(
                        label()
                            .text("Watched Folders")
                            .font_size(23.0)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        label()
                            .text("Indexed music in each library location")
                            .color(colors.muted_foreground),
                    );
                if data.folders.is_empty() {
                    content = content.child(
                        label()
                            .text("Add a music folder to start building your library.")
                            .color(colors.muted_foreground),
                    );
                }
                for folder in data.folders {
                    content = content.child(
                        rect()
                            .width(Size::fill())
                            .padding(16.0)
                            .spacing(8.0)
                            .corner_radius(12.0)
                            .background(colors.surface_subtle)
                            .border(Border::new().width(1.0).fill(colors.border_subtle))
                            .child(label().text(folder.path).font_weight(FontWeight::BOLD))
                            .child(
                                label()
                                    .text(format!(
                                        "{} tracks · {:.1} MB",
                                        folder.track_count,
                                        folder.total_size as f64 / 1_000_000.0
                                    ))
                                    .color(colors.muted_foreground),
                            ),
                    );
                }
            }
        }
        content = content
            .child(
                Button::new()
                    .flat()
                    .child(if dark {
                        "Switch to light theme"
                    } else {
                        "Switch to dark theme"
                    })
                    .on_press(move |_| {
                        let preference = if controller.preference() == ThemePreference::Dark {
                            ThemePreference::Light
                        } else {
                            ThemePreference::Dark
                        };
                        theme_error.set(
                            controller
                                .set(preference)
                                .err()
                                .map(|error| error.to_string()),
                        );
                    }),
            )
            .maybe_child(
                theme_error
                    .read()
                    .clone()
                    .map(|message| label().text(message).color(colors.destructive)),
            );
        crate::ui::components::SmoothScroll::new().child(content)
    }
}
