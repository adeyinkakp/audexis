use super::*;
use audexis_audio::lyrics_preview::{Action, Preview as AudioPreview, Status};
use std::sync::Mutex;
#[derive(Clone)]
pub struct Preview {
    pub status: State<Status>,
    session: Arc<Mutex<Option<AudioPreview>>>,
    runtime: Handle,
    path: String,
    services: Arc<AppServices>,
}
impl PartialEq for Preview {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}
impl Preview {
    pub fn command(&self, action: Action, position: u64) {
        let session = self.session.clone();
        let path = self.path.clone();
        let services = self.services.clone();
        let runtime = self.runtime.clone();
        let mut status = self.status;
        let retry = status.peek().error.is_some();
        spawn(async move {
            let result = runtime
                .spawn_blocking(move || {
                    let mut slot = session
                        .lock()
                        .map_err(|_| "Preview lock failed".to_string())?;
                    if retry {
                        slot.take();
                    }
                    if slot.is_none() {
                        let equalizer = services
                            .audio
                            .as_ref()
                            .and_then(|a| a.lock().ok().map(|a| a.equalizer.clone()))
                            .unwrap_or_default();
                        let (preview, _) =
                            AudioPreview::open(uuid::Uuid::new_v4().to_string(), path, equalizer)?;
                        if let Some(player) = &services.player {
                            let _ = player.send(audexis_audio::PlayerCmd::Pause);
                        }
                        *slot = Some(preview);
                    }
                    slot.as_ref().unwrap().request(action, position)
                })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);
            match result {
                Ok(next) => status.set(next),
                Err(error) => status.write().error = Some(error),
            }
        });
    }
}
pub fn use_preview(path: String) -> Preview {
    let status = use_state(Status::default);
    let session = use_hook(|| Arc::new(Mutex::new(None::<AudioPreview>)));
    let runtime = use_consume::<Handle>();
    let services = use_consume::<Arc<AppServices>>();
    use_hook({
        let session = session.clone();
        let runtime = runtime.clone();
        let mut status = status;
        move || {
            spawn(async move {
                loop {
                    let _ = runtime
                        .spawn(async {
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        })
                        .await;
                    let session = session.clone();
                    if let Ok(Some(Ok(next))) = runtime
                        .spawn_blocking(move || {
                            session.lock().ok().and_then(|slot| {
                                slot.as_ref().map(|p| p.request(Action::Status, 0))
                            })
                        })
                        .await
                    {
                        status.set(next);
                    }
                }
            });
        }
    });
    use_drop({
        let session = session.clone();
        let runtime = runtime.clone();
        move || {
            runtime.spawn_blocking(move || {
                if let Ok(mut slot) = session.lock() {
                    slot.take();
                }
            });
        }
    });
    Preview {
        status,
        session,
        runtime,
        path,
        services,
    }
}
#[derive(PartialEq)]
pub struct Controls {
    pub preview: Preview,
}
impl Component for Controls {
    fn render(&self) -> impl IntoElement {
        let status = self.preview.status.read().clone();
        let play = self.preview.clone();
        let slider = self.preview.clone();
        let duration = status.duration_ms;
        let colors = super::super::theme::use_audexis_colors();
        let session = use_consume::<State<Session>>();
        let s = session.read();
        let title = s.text(FrameKey::Title);
        let artist = s.text(FrameKey::Artist);
        let time = |ms: u64| format!("{}:{:02}", ms / 60000, ms / 1000 % 60);
        rect()
            .width(Size::fill())
            .padding(16.)
            .corner_radius(12.)
            .background(colors.surface_glass)
            .spacing(12.)
            .child(label().text(title).font_size(16.))
            .child(
                label()
                    .text(artist)
                    .font_size(12.)
                    .color(colors.muted_foreground),
            )
            .child(
                rect()
                    .horizontal()
                    .spacing(8.)
                    .child(
                        Button::new()
                            .child(if status.playing {
                                "Pause preview"
                            } else {
                                "Play preview"
                            })
                            .on_press(move |_| {
                                play.command(
                                    if status.playing {
                                        Action::Pause
                                    } else {
                                        Action::Play
                                    },
                                    0,
                                )
                            }),
                    )
                    .child(
                        label()
                            .text(format!(
                                "{} / {}",
                                time(status.position_ms),
                                time(status.duration_ms)
                            ))
                            .font_size(12.)
                            .color(colors.muted_foreground),
                    ),
            )
            .child(
                Slider::new(move |percent: f64| {
                    slider.command(Action::Seek, (percent / 100.0 * duration as f64) as u64)
                })
                .thumb_background(colors.transparent)
                .thumb_inner_background(colors.transparent)
                .enabled(duration > 0)
                .value(if duration > 0 {
                    status.position_ms as f64 / duration as f64 * 100.0
                } else {
                    0.0
                }),
            )
            .maybe_child(status.error.map(|e| label().text(e)))
    }
}
