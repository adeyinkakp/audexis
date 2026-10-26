use crate::{services::AppServices, ui::theme::use_audexis_colors};
use freya::{icons::lucide, prelude::*};
use std::{sync::Arc, time::Duration};

#[derive(PartialEq)]
pub struct Artwork {
    pub file_id: i64,
    pub path: String,
    pub size: f32,
}
impl Component for Artwork {
    fn render(&self) -> impl IntoElement {
        let (visible, on_sized) = super::visibility::use_visibility_margin(512.);
        let mut retained = use_state(|| false);
        let runtime = use_consume::<tokio::runtime::Handle>();
        let mut release = use_hook(move || {
            FutureTask::create(move || {
                let _runtime = runtime.enter();
                let delay = tokio::time::sleep(Duration::from_secs(30));
                async move {
                    delay.await;
                    retained.set_if_modified(false);
                }
            })
        });
        use_side_effect_with_deps(&visible, move |visible| {
            release.cancel();
            if *visible {
                retained.set_if_modified(true);
            } else if *retained.peek() {
                release.start();
            }
        });
        rect()
            .width(Size::px(self.size))
            .height(Size::px(self.size))
            .on_sized(on_sized)
            .child(if visible || *retained.read() {
                VisibleArtwork {
                    file_id: self.file_id,
                    path: self.path.clone(),
                    size: self.size,
                }
                .into_element()
            } else {
                ArtworkPlaceholder(self.size).into_element()
            })
    }
}

#[derive(PartialEq)]
struct VisibleArtwork {
    file_id: i64,
    path: String,
    size: f32,
}
impl Component for VisibleArtwork {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let files = crate::ui::hooks::use_files(&[self.file_id]);
        let path = files
            .data
            .as_ref()
            .and_then(|files| files.first())
            .map(|file| file.path.clone())
            .unwrap_or_else(|| self.path.clone());
        let revision = crate::ui::hooks::use_library().artwork_revision();
        let scale = *Platform::get().scale_factor.read();
        let edge = audexis_core::artwork::thumbnail_edge((self.size as f64 * scale).ceil() as u32);
        let cached = services.artwork.cached(self.file_id, &path, edge);
        let mut retained = use_state(|| None::<Arc<audexis_core::artwork::Artwork>>);
        let mut retained_key = use_state(|| None::<(i64, String)>);
        let key = (self.file_id, path, revision, edge);
        let source = use_reactive(&key);
        let mut loader = use_hook(move || {
            FutureTask::create(move || {
                let (id, path, _, edge) = source.read().clone();
                let cache = services.artwork.clone();
                let cloud = path
                    .starts_with("cloud-library://")
                    .then(|| services.cloud_context());
                let runtime = runtime.clone();
                async move {
                    let task = runtime.spawn(async move {
                        if let Some(context) = cloud {
                            let context = context?;
                            if let Err(error) = context.ensure_library_artwork(id, &path).await {
                                tracing::warn!(file_id=id, reason=%error, "Could not load cloud artwork");
                                return Err(error.to_string());
                            }
                            cache.invalidate_source(id, &path).await;
                        }
                        cache.load_sized(id, path, edge).await
                    });

                    let _abort = AbortOnDrop(task.abort_handle());
                    task.await.map_err(|error| error.to_string())?
                }
            })
        });
        use_side_effect_with_deps(&key, move |(id, path, _, _)| {
            let identity = (*id, path.clone());
            if retained_key.peek().as_ref() != Some(&identity) {
                retained_key.set(Some(identity));
                retained.set(None);
            }
            loader.cancel();
            loader.start();
        });
        let state = loader.state();
        let completed = state
            .try_as_fulfilled()
            .and_then(|result| result.as_ref().ok())
            .cloned();
        let image_key = completed
            .as_ref()
            .and_then(|artwork| artwork.as_ref())
            .map(|artwork| artwork.hash);
        use_side_effect_with_deps(&(image_key, completed.is_some()), move |_| {
            let state = loader.state();
            if let Some(Ok(artwork)) = state.try_as_fulfilled() {
                retained.set(artwork.clone());
            }
        });
        let artwork = match state.try_as_fulfilled() {
            Some(Ok(artwork)) => artwork.clone(),
            Some(Err(_)) | None => retained.read().clone().or(cached),
        };
        match artwork {
            Some(artwork) => ImageViewer::new((artwork.hash, artwork.bytes.clone()))
                .width(Size::px(self.size))
                .height(Size::px(self.size))
                .aspect_ratio(AspectRatio::Max)
                .image_cover(ImageCover::Center)
                .corner_radius(8.0)
                .asset_age(Duration::from_secs(60))
                .a11y_alt("Album artwork")
                .loading_placeholder(ArtworkPlaceholder(self.size))
                .into_element(),
            _ => ArtworkPlaceholder(self.size).into_element(),
        }
    }
}

struct AbortOnDrop(tokio::task::AbortHandle);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[derive(PartialEq)]
struct ArtworkPlaceholder(f32);
impl Component for ArtworkPlaceholder {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();

        rect()
            .width(Size::px(self.0))
            .height(Size::px(self.0))
            .corner_radius(8.0)
            .background(colors.muted)
            .center()
            .child(
                SvgViewer::new(("music", lucide::music_2()))
                    .color(colors.muted_foreground)
                    .width(Size::px(self.0.min(28.0)))
                    .height(Size::px(self.0.min(28.0))),
            )
            .into_element()
    }
}
