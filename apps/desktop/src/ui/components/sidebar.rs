use freya::prelude::*;
use std::sync::Arc;

use crate::ui::{routes::Route, theme::use_audexis_colors};

#[derive(PartialEq)]
pub struct Sidebar;

impl Component for Sidebar {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<crate::services::AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();

        rect()
            .width(Size::px(220.0))
            .height(Size::fill())
            .background(colors.background)
            .padding(16.0)
            .spacing(24.0)
            .child(
                ActivableRoute::new(
                    Route::Home,
                    Link::new(Route::Home).child(
                        SideBarItem::new()
                            .color(colors.foreground)
                            .background(colors.background)
                            .hover_background(colors.hover)
                            .active_background(colors.active)
                            .focus_border_fill(colors.ring)
                            .child("Songs"),
                    ),
                )
                .exact(true),
            )
            .child(
                ActivableRoute::new(
                    Route::Songs,
                    Link::new(Route::Songs).child(
                        SideBarItem::new()
                            .color(colors.foreground)
                            .background(colors.background)
                            .hover_background(colors.hover)
                            .active_background(colors.active)
                            .focus_border_fill(colors.ring)
                            .child("Songs"),
                    ),
                )
                .exact(true),
            )
            .child(
                SideBarItem::new()
                    .color(colors.foreground)
                    .background(colors.background)
                    .hover_background(colors.hover)
                    .active_background(colors.active)
                    .focus_border_fill(colors.ring)
                    .child(label().text("Settings"))
                    .on_press(move |_| {
                        crate::ui::launch_settings(services.clone(), runtime.clone());
                    }),
            )
    }
}
