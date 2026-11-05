use super::visibility::use_visibility;
use crate::ui::theme::use_audexis_colors;
use freya::prelude::*;

#[derive(PartialEq)]
pub(crate) struct InfiniteScroll {
    pub has_more: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub progress: usize,
    pub on_load: EventHandler<()>,
}
impl Component for InfiniteScroll {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let (visible, on_sized) = use_visibility();
        let mut requested = use_state(|| None::<usize>);
        let progress = self.progress;
        let ready = visible && self.has_more && !self.loading && self.error.is_none();
        let load = self.on_load.clone();
        use_side_effect_with_deps(&(ready, progress), move |(ready, progress)| {
            if *ready && *requested.peek() != Some(*progress) {
                requested.set(Some(*progress));
                load.call(());
            }
        });
        let retry = self.on_load.clone();
        rect()
            .width(Size::fill())
            .height(Size::px(if self.has_more || self.error.is_some() {
                48.
            } else {
                0.
            }))
            .center()
            .on_sized(on_sized)
            .maybe_child(
                self.loading
                    .then(|| label().text("Loading more…").color(colors.muted_foreground)),
            )
            .maybe_child(self.error.clone().map(|error| {
                Button::new()
                    .child(format!("{error} · Try again"))
                    .on_press(move |_| retry.call(()))
            }))
    }
    fn render_key(&self) -> DiffKey {
        (&("infinite-scroll", self.progress)).into()
    }
}
