use super::track_selection::{SelectableTrack, TrackSelectionScope, song_items};
use crate::{
    services::AppServices,
    ui::{
        components::{Artwork, media::MediaTarget},
        hooks::{use_library, use_playback},
        theme::use_audexis_colors,
    },
};
use audexis_audio::{PlayerCmd, queue_track::UnresolvedTrack};
use audexis_core::database::songs::SongRow;
use freya::prelude::*;
use std::sync::Arc;

#[derive(PartialEq)]
pub(crate) struct TrackList {
    pub tracks: Arc<Vec<SongRow>>,
    pub playlist: Option<i64>,
    pub on_change: EventHandler<()>,
}
impl Component for TrackList {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let selection_items = song_items(&self.tracks, self.playlist);
        let (range, on_sized) = super::visibility::use_visible_rows(self.tracks.len(), 64.);
        let playback = use_playback();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let library = use_library();
        let mut error = use_state(|| None::<String>);
        let play = playback.clone();
        let all = self.tracks.clone();
        let mut body = rect()
            .width(Size::fill())
            .spacing(8.)
            .child(
                rect()
                    .horizontal()
                    .spacing(12.)
                    .cross_align(Alignment::Center)
                    .child(
                        Button::new()
                            .filled()
                            .enabled(!self.tracks.is_empty() && playback.available())
                            .child("Play")
                            .on_press(move |_| {
                                play.send(PlayerCmd::Load {
                                    tracks: playback_tracks(&all),
                                })
                            }),
                    )
                    .child(
                        label()
                            .text(format!("{} songs", self.tracks.len()))
                            .font_size(12.)
                            .color(colors.muted_foreground),
                    ),
            )
            .maybe_child(
                error
                    .read()
                    .clone()
                    .map(|e| label().text(e).color(colors.destructive)),
            )
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .padding((8., 12.))
                    .spacing(12.)
                    .child(label().text("#").width(Size::px(28.)))
                    .child(label().text("Title").width(Size::flex(2.)))
                    .child(label().text("Artist").width(Size::flex(1.)))
                    .child(label().text("Album").width(Size::flex(1.)))
                    .child(label().text("Time").width(Size::px(90.)))
                    .color(colors.muted_foreground),
            );
        if self.tracks.is_empty() {
            body = body.child(
                label()
                    .text("No songs found.")
                    .color(colors.muted_foreground)
                    .margin(20.),
            );
        }
        let mut rows = rect().width(Size::fill()).on_sized(on_sized).child(
            rect()
                .key("before")
                .height(Size::px(range.start as f32 * 64.)),
        );
        for index in range.clone() {
            let song = &self.tracks[index];
            let play = playback.clone();
            let tracks = self.tracks.clone();
            let id = song.id;
            let loved = song.loved;
            let db = services.db.clone();
            let runtime = runtime.clone();
            let remove_runtime = runtime.clone();
            let library = library.clone();
            rows = rows.child(
                rect()
                    .key((
                        "track-row",
                        self.playlist,
                        song.playlist_ord,
                        song.id,
                        index,
                    ))
                    .width(Size::fill())
                    .height(Size::px(64.))
                    .child(
                        SelectableTrack::new(
                            selection_items[index].clone(),
                            MediaTarget::track(id, song.path.clone()),
                            EventHandler::new(move |()| {
                                play.send(PlayerCmd::LoadAt {
                                    tracks: playback_tracks(&tracks),
                                    index,
                                })
                            }),
                        )
                        .background(if index % 2 == 0 {
                            colors.surface_subtle
                        } else {
                            colors.background
                        })
                        .child(
                            rect()
                                .horizontal()
                                .width(Size::fill())
                                .height(Size::px(56.))
                                .content(Content::Flex)
                                .cross_align(Alignment::Center)
                                .padding((6., 12.))
                                .spacing(12.)
                                .child(
                                    label()
                                        .text((index + 1).to_string())
                                        .width(Size::px(28.))
                                        .color(colors.muted_foreground),
                                )
                                .child(
                                    rect()
                                        .horizontal()
                                        .width(Size::flex(2.))
                                        .spacing(12.)
                                        .cross_align(Alignment::Center)
                                        .child(Artwork {
                                            file_id: id,
                                            path: song.path.clone(),
                                            size: 40.,
                                        })
                                        .child(
                                            label()
                                                .text(crate::ui::display_text(song.title.clone()))
                                                .max_lines(1)
                                                .text_overflow(TextOverflow::Ellipsis),
                                        ),
                                )
                                .child(
                                    label()
                                        .text(crate::ui::display_text(song.artist.clone()))
                                        .width(Size::flex(1.))
                                        .max_lines(1)
                                        .text_overflow(TextOverflow::Ellipsis)
                                        .color(colors.muted_foreground),
                                )
                                .child(
                                    label()
                                        .text(crate::ui::display_text(song.album.clone()))
                                        .width(Size::flex(1.))
                                        .max_lines(1)
                                        .text_overflow(TextOverflow::Ellipsis)
                                        .color(colors.muted_foreground),
                                )
                                .child(
                                    label()
                                        .text(
                                            song.duration_ms
                                                .map(|ms| {
                                                    format!("{}:{:02}", ms / 60000, ms / 1000 % 60)
                                                })
                                                .unwrap_or_else(|| "—".into()),
                                        )
                                        .width(Size::px(42.))
                                        .font_size(12.),
                                )
                                .child(
                                    Button::new()
                                        .flat()
                                        .compact()
                                        .child(if loved { "♥" } else { "♡" })
                                        .on_press(move |event: Event<PressEventData>| {
                                            event.stop_propagation();
                                            let db = db.clone();
                                            let runtime = runtime.clone();
                                            let library = library.clone();
                                            spawn(async move {
                                                match runtime
                                                    .spawn(async move {
                                                        db.set_favorite(id, !loved).await
                                                    })
                                                    .await
                                                {
                                                    Ok(Ok(())) => library.refresh(),
                                                    other => error.set(Some(format!(
                                                        "Could not update favorite: {other:?}"
                                                    ))),
                                                }
                                            });
                                        }),
                                )
                                .maybe_child(self.playlist.zip(song.playlist_ord).map(
                                    |(playlist, ord)| {
                                        let db = services.db.clone();
                                        let runtime = remove_runtime.clone();
                                        let callback = self.on_change.clone();
                                        Button::new().flat().compact().child("Remove").on_press(
                                            move |event: Event<PressEventData>| {
                                                event.stop_propagation();
                                                let db = db.clone();
                                                let runtime = runtime.clone();
                                                let callback = callback.clone();
                                                spawn(async move {
                                                    match runtime
                                                        .spawn(async move {
                                                            db.remove_playlist_occurrence(
                                                                playlist, ord, id,
                                                            )
                                                            .await
                                                        })
                                                        .await
                                                    {
                                                        Ok(Ok(())) => callback.call(()),
                                                        other => error.set(Some(format!(
                                                            "Could not remove song: {other:?}"
                                                        ))),
                                                    }
                                                });
                                            },
                                        )
                                    },
                                ))
                                .into_element(),
                        ),
                    ),
            );
        }
        let count = self.tracks.len();
        let body = body.child(
            rows.child(
                rect()
                    .key("after")
                    .height(Size::px((count - range.end) as f32 * 64.)),
            ),
        );
        TrackSelectionScope {
            items: selection_items,
            child: body.into_element(),
            fill: false,
            on_select_all: None,
        }
    }
}

fn playback_tracks(tracks: &[SongRow]) -> Vec<UnresolvedTrack> {
    tracks
        .iter()
        .map(|song| UnresolvedTrack {
            id: song.id,
            path: song.path.clone(),
            occurrence: song.playlist_ord,
        })
        .collect()
}
