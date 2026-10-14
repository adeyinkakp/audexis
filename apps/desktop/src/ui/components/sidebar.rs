use freya::prelude::*;

use crate::ui::{routes::Route, theme::use_audexis_colors};

#[derive(PartialEq)]
pub struct Sidebar;

impl Component for Sidebar {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();

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
    }
}
