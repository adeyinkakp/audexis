use crate::ui::{
    components::{Artwork, FavoriteButton},
    hooks::use_track_metadata,
    theme::use_audexis_colors,
};
use freya::prelude::*;

#[derive(PartialEq)]
pub(super) struct PlayerDetails {
    pub id: i64,
    pub path: Option<String>,
    pub compact: bool,
    pub tight: bool,
    pub art_size: f32,
}
impl Component for PlayerDetails {
    fn render(&self) -> impl IntoElement {
        let compact = self.compact;
        let tight = self.tight;
        let art_size = self.art_size;
        let colors = use_audexis_colors();
        let metadata = use_track_metadata(self.id, self.path.as_deref());
        let artist = if metadata.available {
            metadata.artist
        } else {
            "Choose a song from your library".into()
        };
        let title = metadata.title;
        let album = metadata.album;
        let text = rect()
            .width(Size::fill())
            .cross_align(if compact {
                Alignment::Start
            } else {
                Alignment::Center
            })
            .spacing(5.)
            .child(
                label()
                    .text(crate::ui::display_text(title))
                    .width(Size::fill())
                    .text_align(if compact {
                        TextAlign::Left
                    } else {
                        TextAlign::Center
                    })
                    .max_lines(1)
                    .text_overflow(TextOverflow::Ellipsis)
                    .font_size(if compact {
                        16.
                    } else if tight {
                        24.
                    } else {
                        30.
                    })
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                label()
                    .text(crate::ui::display_text(artist))
                    .width(Size::fill())
                    .text_align(if compact {
                        TextAlign::Left
                    } else {
                        TextAlign::Center
                    })
                    .max_lines(1)
                    .text_overflow(TextOverflow::Ellipsis)
                    .font_size(if compact {
                        12.
                    } else if tight {
                        16.
                    } else {
                        18.
                    })
                    .color(colors.content_secondary),
            )
            .maybe_child((!album.is_empty()).then(|| {
                label()
                    .text(crate::ui::display_text(album))
                    .width(Size::fill())
                    .text_align(if compact {
                        TextAlign::Left
                    } else {
                        TextAlign::Center
                    })
                    .max_lines(1)
                    .font_size(if compact {
                        11.
                    } else if tight {
                        12.
                    } else {
                        14.
                    })
                    .color(colors.content_faint)
            }));
        let details = if compact {
            rect()
                .width(Size::fill())
                .horizontal()
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .spacing(16.)
                .child(Artwork {
                    file_id: self.id,
                    path: self.path.clone().unwrap_or_default(),
                    size: art_size,
                })
                .child(rect().width(Size::flex(1.)).child(text))
                .child(FavoriteButton { id: self.id })
                .into_element()
        } else {
            rect()
                .width(Size::fill())
                .spacing(if tight { 12. } else { 20. })
                .cross_align(Alignment::Center)
                .child(
                    rect()
                        .corner_radius(24.)
                        .overflow(Overflow::Clip)
                        .shadow(Shadow::new().y(16.).blur(32.).color(colors.shade))
                        .child(Artwork {
                            file_id: self.id,
                            path: self.path.clone().unwrap_or_default(),
                            size: art_size,
                        }),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .content(Content::Flex)
                        .cross_align(Alignment::Center)
                        .child(rect().width(Size::px(36.)))
                        .child(rect().width(Size::flex(1.)).child(text))
                        .child(FavoriteButton { id: self.id }),
                )
                .into_element()
        };
        details
    }
}
