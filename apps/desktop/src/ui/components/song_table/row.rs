use super::ROW_HEIGHT;
use crate::ui::components::track_selection::SelectableTrack;
use crate::ui::{
    components::media::{MediaTarget, SelectedTrack},
    hooks::use_playback,
    theme::use_audexis_colors,
};
use audexis_core::{
    config::TableColumn,
    database::songs::{SongQuery, SongRow},
};
use freya::prelude::*;

#[derive(PartialEq)]
pub(crate) struct SongTableRow {
    pub(crate) columns: Vec<TableColumn>,
    pub(crate) query: SongQuery,
    pub(crate) song: SongRow,
    pub(crate) index: usize,
    pub(crate) item: SelectedTrack,
    pub(crate) load_more: State<Callback<(), ()>>,
    pub(crate) near_end: bool,
    pub(crate) busy: State<bool>,
}
impl Component for SongTableRow {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let playback = use_playback();
        let current = playback.state.read().id == self.song.id;
        let target =
            MediaTarget::library_track(self.song.id, self.song.path.clone(), self.query.clone());
        let command_target = target.clone();
        let mut load_more = self.load_more;
        use_side_effect_with_deps(&(self.near_end, *self.busy.read()), move |(near, busy)| {
            if *near && !*busy {
                load_more.write().call(());
            }
        });
        let mut row = rect()
            .width(Size::fill())
            .height(Size::px(ROW_HEIGHT))
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .padding((0.0, 8.0))
            .child(
                rect()
                    .text_align(TextAlign::Center)
                    .child(if current {
                        "▶".into()
                    } else {
                        if self.song.download_path.is_some() {
                            "✓".to_string()
                        } else if self.song.path.starts_with("cloud-library://") {
                            "☁".to_string()
                        } else {
                            "".to_string()
                        }
                    })
                    .width(Size::px(34.0))
                    .color(if self.song.download_path.is_some() {
                        colors.primary
                    } else {
                        colors.muted_foreground
                    }),
            );
        for col in &self.columns {
            let value = self.song.column_text(&col.column);
            row = row.child(
                rect()
                    .width(Size::px(col.width as f32))
                    .padding((0.0, 6.0))
                    .child(
                        label()
                            .text(crate::ui::display_text(value))
                            .width(Size::fill())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(12.0)
                            .color(if current {
                                colors.primary
                            } else {
                                colors.foreground
                            }),
                    ),
            );
        }
        SelectableTrack::new(
            self.item.clone(),
            target,
            EventHandler::new(move |()| command_target.play(&playback)),
        )
        .background(if self.index % 2 == 0 {
            colors.surface_subtle
        } else {
            Color::TRANSPARENT
        })
        .child(row)
    }
}
