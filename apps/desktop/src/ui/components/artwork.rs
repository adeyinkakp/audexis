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
                    backdrop: None,
                }
                .into_element()
            } else {
                ArtworkPlaceholder(self.size).into_element()
            })
    }
}

#[derive(PartialEq)]
pub(crate) struct ArtworkBackdrop {
    pub file_id: i64,
    pub path: String,
    pub width: f32,
    pub height: f32,
}
impl Component for ArtworkBackdrop {
    fn render(&self) -> impl IntoElement {
        VisibleArtwork {
            file_id: self.file_id,
            path: self.path.clone(),
            size: self.width.max(self.height),
            backdrop: Some((self.width, self.height)),
        }
    }
}

#[derive(PartialEq)]
struct VisibleArtwork {
    file_id: i64,
    path: String,
    size: f32,
    backdrop: Option<(f32, f32)>,
}
impl Component for VisibleArtwork {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
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
        let backdrop = self.backdrop.is_some();
        let cached = if backdrop {
            None
        } else {
            services.artwork.cached(self.file_id, &path, edge)
        };
        let mut retained = use_state(|| None::<Arc<audexis_core::artwork::Artwork>>);
        let mut retained_key = use_state(|| None::<(i64, String)>);
        let key = (self.file_id, path, revision, edge, backdrop);
        let source = use_reactive(&key);
        let mut loader = use_hook(move || {
            FutureTask::create(move || {
                let (id, path, _, edge, backdrop) = source.read().clone();
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
                        let artwork = cache.load_sized(id, path, edge).await?;
                        if backdrop {
                            if let Some(artwork) = artwork {
                                return tokio::task::spawn_blocking(move || blurred_backdrop(artwork))
                                    .await.map_err(|error| error.to_string())?.map(Some);
                            }
                        }
                        Ok(artwork)
                    });

                    let _abort = AbortOnDrop(task.abort_handle());
                    task.await.map_err(|error| error.to_string())?
                }
            })
        });
        use_side_effect_with_deps(&key, move |(id, path, _, _, _)| {
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
        let (width, height) = self.backdrop.unwrap_or((self.size, self.size));
        let placeholder = if self.backdrop.is_some() {
            rect()
                .width(Size::px(width))
                .height(Size::px(height))
                .background(colors.muted)
                .into_element()
        } else {
            ArtworkPlaceholder(self.size).into_element()
        };
        match artwork {
            Some(artwork) => ImageViewer::new((artwork.hash, artwork.bytes.clone()))
                .width(Size::px(width))
                .height(Size::px(height))
                .aspect_ratio(AspectRatio::Max)
                .image_cover(ImageCover::Center)
                .corner_radius(if self.backdrop.is_some() { 0. } else { 8. })
                .asset_age(Duration::from_secs(60))
                .a11y_alt("Album artwork")
                .loading_placeholder(placeholder)
                .into_element(),
            _ => placeholder,
        }
    }
}

fn blurred_backdrop(
    artwork: Arc<audexis_core::artwork::Artwork>,
) -> Result<Arc<audexis_core::artwork::Artwork>, String> {
    use sha2::{Digest, Sha256};
    let image = image::load_from_memory(&artwork.bytes)
        .map_err(|error| error.to_string())?
        .thumbnail(96, 96)
        .blur(10.);
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    let mut hash = Sha256::new();
    hash.update(artwork.hash);
    hash.update(b"blurred-backdrop");
    Ok(Arc::new(audexis_core::artwork::Artwork {
        hash: hash.finalize().into(),
        bytes: bytes.into_inner().into(),
    }))
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
