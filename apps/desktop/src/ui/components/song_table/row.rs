use super::ROW_HEIGHT;
use crate::ui::{
    components::media::{MediaItem, MediaTarget},
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
    pub(crate) selected: State<Option<i64>>,
    pub(crate) load_more: State<Callback<(), ()>>,
    pub(crate) near_end: bool,
}
impl Component for SongTableRow {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let playback = use_playback();
        let current = playback.state.read().id == self.song.id;
        let mut selected = self.selected;
        let id = self.song.id;
        let target = MediaTarget::library_track(id, self.song.path.clone(), self.query.clone());
        let command_target = target.clone();
        let mut load_more = self.load_more;
        use_side_effect_with_deps(&self.near_end, move |near| {
            if *near {
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
            .background(if *selected.read() == Some(id) {
                colors.primary_washed
            } else if self.index % 2 == 0 {
                colors.surface_subtle
            } else {
                Color::TRANSPARENT
            })
            .a11y_alt(format!(
                "{}, {}, {}",
                self.song.title, self.song.artist, self.song.album
            ))
            .a11y_focusable(true)
            .on_press(move |event: Event<PressEventData>| {
                selected.set(Some(id));
                let play = match event.data() {
                    PressEventData::Mouse(mouse) => {
                        EventsCombos::pressed(mouse.global_location).is_double()
                    }
                    PressEventData::Keyboard(_) => true,
                    PressEventData::Touch(_) => false,
                };
                if play {
                    command_target.play(&playback);
                }
            })
            .child(
                label()
                    .text(if current {
                        "▶".into()
                    } else {
                        (self.index + 1).to_string()
                    })
                    .width(Size::px(34.0))
                    .font_size(11.0)
                    .color(colors.muted_foreground),
            );
        for col in &self.columns {
            let value = self.song.column_text(&col.column);
            row = row.child(
                rect()
                    .width(Size::px(col.width as f32))
                    .padding((0.0, 6.0))
                    .child(
                        label()
                            .text(value)
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
        MediaItem::new(target).child(row)
    }
}
