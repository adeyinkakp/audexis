use super::*;

pub(super) fn playback_settings(
    settings_services: Arc<AppServices>,
    custom_runtime: tokio::runtime::Handle,
    mut quality: State<audexis_core::config::CloudAudioQuality>,
    mut cache_mb: State<u64>,
    mut sync_error: State<Option<String>>,
    colors: crate::ui::theme::AudexisColors,
) -> impl IntoElement {
    let mut quality_options = rect().horizontal().spacing(8.);
    for (value, title) in [
        (
            audexis_core::config::CloudAudioQuality::Original,
            "Original",
        ),
        (
            audexis_core::config::CloudAudioQuality::Balanced,
            "Data saver · 128 kbps",
        ),
        (
            audexis_core::config::CloudAudioQuality::High,
            "High · 256 kbps",
        ),
    ] {
        let services = settings_services.clone();
        quality_options = quality_options.child(
            Button::new()
                .outline()
                .enabled(*quality.read() != value)
                .child(title)
                .on_press(move |_| {
                    match services.update_config(|c| c.cloud_audio_quality = value) {
                        Ok(()) => {
                            quality.set(value);
                            sync_error.set(None);
                        }
                        Err(e) => sync_error.set(Some(e.to_string())),
                    }
                }),
        );
    }
    let mut cache_options = rect().horizontal().spacing(8.);
    for (value, title) in [(0, "Off"), (512, "512 MB"), (2048, "2 GB"), (5120, "5 GB")] {
        let services = settings_services.clone();
        cache_options = cache_options.child(
            Button::new()
                .outline()
                .enabled(*cache_mb.read() != value)
                .child(title)
                .on_press(
                    move |_| match services.update_config(|c| c.playback_cache_mb = value) {
                        Ok(()) => {
                            cache_mb.set(value);
                            sync_error.set(None);
                        }
                        Err(e) => sync_error.set(Some(e.to_string())),
                    },
                ),
        );
    }
    let clear_services = settings_services.clone();
    let clear_runtime = custom_runtime.clone();
    rect().spacing(12.)
            .child(label().text("Cloud playback").font_weight(FontWeight::BOLD))
            .child(quality_options)
            .child(label().text("Lower-data copies require server support. While a copy is preparing, playback uses the original. Quality changes apply to the next track.").font_size(12.).color(colors.muted_foreground))
            .child(label().text("Recently played cache"))
            .child(cache_options)
            .child(label().text("Older cached audio is removed automatically. Offline downloads are kept separately.").font_size(12.).color(colors.muted_foreground))
            .child(Button::new().outline().child("Clear playback cache").on_press(move |_| {
                match clear_services.cloud_context() {
                    Ok(context) => {
                        let path = context.cache_dir.join(&context.connection_id).join("streaming");
                        let task = clear_runtime.spawn_blocking(move || audexis_audio::streaming::clear_playback_cache(&path));
                        spawn(async move { match task.await { Ok(Ok(())) => sync_error.set(None), Ok(Err(e)) => sync_error.set(Some(e.to_string())), Err(e) => sync_error.set(Some(e.to_string())) } });
                    },
                    Err(e) => sync_error.set(Some(e)),
                }
            }))
}
