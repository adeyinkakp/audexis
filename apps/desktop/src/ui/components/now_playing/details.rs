use super::*;

#[derive(PartialEq)]
pub(super) struct TrackDetails {
    pub(super) id: i64,
    pub(super) path: Option<String>,
    pub(super) song_menu: EventHandler<Event<PressEventData>>,
}
impl Component for TrackDetails {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let song_menu = self.song_menu.clone();
        let songs = crate::ui::hooks::use_songs(&[self.id]);
        let info = songs.data.as_ref().and_then(|songs| songs.first()).cloned();
        let fallback = self
            .path
            .as_deref()
            .and_then(|path| Path::new(path).file_stem())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Nothing is playing".into());
        let mut title = info
            .as_ref()
            .map(|info| info.title.clone())
            .unwrap_or(fallback);
        let mut subtitle = info
            .map(|info| {
                [info.artist, info.album]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · ")
            })
            .unwrap_or_else(|| {
                if self.path.is_some() {
                    "Local audio".into()
                } else {
                    String::new()
                }
            });
        let cloud = use_playback().cloud_track;
        if let Some((path, track)) = &*cloud.read() {
            if self.path.as_ref() == Some(path) {
                title = track.title();
                subtitle = track.artist();
            }
        }
        rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(8.0)
            .child(Artwork {
                file_id: self.id,
                path: self.path.clone().unwrap_or_default(),
                size: 36.0,
            })
            .child(
                rect()
                    .width(Size::flex(1.0))
                    .spacing(2.0)
                    .child(
                        label()
                            .text(crate::ui::display_text(title))
                            .width(Size::fill())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(14.0)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        label()
                            .text(crate::ui::display_text(subtitle))
                            .width(Size::fill())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(12.0)
                            .color(colors.muted_foreground),
                    ),
            )
            .child(control(
                "Favorite",
                false,
                SvgViewer::new(("heart", lucide::heart()))
                    .width(Size::px(15.0))
                    .height(Size::px(15.0))
                    .color(colors.muted_foreground),
                |_| {},
            ))
            .child(control(
                "Song options",
                self.path.is_some(),
                SvgViewer::new(("song-menu", lucide::ellipsis()))
                    .width(Size::px(15.0))
                    .height(Size::px(15.0))
                    .color(colors.muted_foreground),
                move |event| song_menu.call(event),
            ))
    }
}
