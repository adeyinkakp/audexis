mod controls;
mod details;
mod header;
mod view;
mod window;

use freya::prelude::*;
use view::PlayerView;
pub(crate) use window::open;

#[derive(Clone, Copy)]
pub(crate) struct ExpandedPlayerContext;
#[derive(PartialEq)]
pub(crate) struct ExpandedPlayer;
impl Component for ExpandedPlayer {
    fn render(&self) -> impl IntoElement {
        use_provide_context(|| ExpandedPlayerContext);
        use_provide_theme(crate::ui::theme::audexis_dark_theme);
        PlayerView {
            compact: false,
            library_window: None,
        }
    }
}
