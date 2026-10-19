use super::*;

#[derive(PartialEq)]
pub(super) struct LyricLines {
    pub(super) lyrics: Arc<audexis_core::lyrics::Lyrics>,
    pub(super) position: u64,
    pub(super) track_key: String,
}
impl Component for LyricLines {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let playback = use_playback();
        let mut follow = use_state(|| true);
        let runtime = use_consume::<tokio::runtime::Handle>();
        let mut resume_follow = use_hook(move || {
            FutureTask::create(move || {
                let _runtime = runtime.enter();
                let delay = tokio::time::sleep(std::time::Duration::from_secs(5));
                async move {
                    delay.await;
                    follow.set_if_modified(true);
                }
            })
        });
        let mut scroll = super::super::lyrics_scroll::use_lyrics_scroll_controller(move || {
            follow.set_if_modified(false);
            resume_follow.start();
        });
        let services = use_consume::<Arc<AppServices>>();
        let reduce_motion = services
            .user_config
            .lock()
            .map(|c| c.reduce_motion)
            .unwrap_or(true);
        let mut viewport = use_state(|| 0.0f32);
        let mut heights = use_state(std::collections::HashMap::<usize, f32>::new);
        let active = self.lyrics.active_index(self.position);
        let target = super::super::lyrics_scroll::follow_target(
            active,
            self.lyrics.lines.len(),
            &heights.read(),
            *viewport.read(),
        );
        let dependencies = (
            self.track_key.clone(),
            active,
            *follow.read(),
            target,
            reduce_motion,
        );
        use_side_effect_with_deps(
            &dependencies,
            move |(_, _, following, target, reduce_motion)| {
                if *following {
                    if let Some(target) = target {
                        scroll.follow_to(*target, *reduce_motion);
                    }
                } else {
                    scroll.stop();
                }
            },
        );
        if self.lyrics.lines.is_empty() {
            return super::super::SmoothScroll::new()
                .child(
                    label()
                        .text(crate::ui::display_text(if self.lyrics.plain.is_empty() {
                            "No embedded lyrics for this song.".into()
                        } else {
                            self.lyrics.plain.clone()
                        }))
                        .font_size(16.0)
                        .color(colors.muted_foreground),
                )
                .into_element();
        }
        let mut lines = rect()
            .on_wheel(move |_| {
                scroll.stop();
                follow.set_if_modified(false);
                resume_follow.start();
            })
            .width(Size::fill())
            .padding((16.0, 0.0, 80.0, 0.0));
        for (index, line) in self.lyrics.lines.iter().enumerate() {
            let playback = playback.clone();
            let time = line.time_ms;
            let gap = line.text.trim().is_empty();
            let content = if gap {
                super::super::lyrics_gap::LyricsGap {
                    active: active == Some(index),
                    position: self.position,
                    start: time,
                    end: self.lyrics.lines[index + 1..]
                        .iter()
                        .find(|line| !line.text.trim().is_empty())
                        .map(|line| line.time_ms),
                    reduce_motion,
                }
                .into_element()
            } else {
                super::super::lyrics_line::LyricText {
                    text: line.text.clone(),
                    active: active == Some(index),
                    reduce_motion,
                }
                .into_element()
            };
            lines = lines.child(
                rect()
                    .width(Size::fill())
                    .on_sized(move |e: Event<SizedEventData>| {
                        let height = e.area.height();
                        if heights.peek().get(&index) != Some(&height) {
                            heights.write().insert(index, height);
                        }
                    })
                    .child(if gap {
                        content
                    } else {
                        rect()
                            .padding((0.0, 0.0, 12.0, 0.0))
                            .child(Button::new().flat().expanded().child(content).on_press(
                                move |_| {
                                    resume_follow.cancel();
                                    follow.set_if_modified(true);
                                    playback
                                        .send(PlayerCmd::SeekMilliseconds { milliseconds: time });
                                },
                            ))
                            .into_element()
                    }),
            );
        }
        rect()
            .expanded()
            .content(Content::Flex)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.0))
                    .on_sized(move |e: Event<SizedEventData>| {
                        viewport.set_if_modified(e.area.height());
                    })
                    .child(
                        VisibilityViewport::new()
                            .child(ScrollView::new_controlled(scroll.controller).child(lines)),
                    ),
            )
            .into_element()
    }
}
