use freya::{prelude::*, router::*};

use super::{
    components::{NowPlaying, PlaybackPanel, Sidebar, VisibilityViewport},
    routes::Route,
    theme::use_audexis_colors,
};

#[derive(PartialEq)]
pub struct Shell;

impl Component for Shell {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let panel_open = crate::ui::hooks::use_playback().panel.read().is_some();
        VisibilityViewport::new().child(
            rect()
                .expanded()
                .native_router()
                .horizontal()
                .content(Content::Flex)
                .child(Sidebar)
                .child(
                    rect()
                        .width(Size::px(1.0))
                        .height(Size::fill())
                        .background(colors.border),
                )
                .child(
                    rect()
                        .width(Size::flex(1.0))
                        .height(Size::fill())
                        .content(Content::Flex)
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::flex(1.0))
                                .padding((0.0, 0.0, 92.0, 0.0))
                                .child(Outlet::<Route>::new()),
                        )
                        .child(NowPlaying),
                )
                .maybe_child(panel_open.then_some(PlaybackPanel)),
        )
    }
}
