use super::*;

pub(super) fn view(
    selected: Section,
    mut draft: State<Draft>,
    save_services: Arc<AppServices>,
    runtime: Handle,
    theme: crate::ui::theme::ThemeController,
    colors: crate::ui::theme::AudexisColors,
) -> impl IntoElement {
    let d = draft.read().clone();
    rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(12.)
        .padding((16., 28.))
        .child(
            label()
                .text(match selected {
                    Section::Keyboard => "Keyboard shortcuts are saved automatically.",
                    Section::Metadata => "Field selections are saved automatically.",
                    Section::CustomFields => "Saved fields apply immediately.",
                    Section::Logs => "Backend logs",
                    Section::Cloud => "Connection credentials are stored securely.",
                    _ => {
                        if d.busy {
                            "Updating your preferences…"
                        } else {
                            "Changes apply when you save."
                        }
                    }
                })
                .font_size(12.)
                .color(colors.muted_foreground)
                .width(Size::flex(1.)),
        )
        .maybe_child(
            matches!(selected, Section::Appearance | Section::Library).then(|| {
                Button::new()
                    .flat()
                    .enabled(d.dirty() && !d.busy)
                    .child("Discard changes")
                    .on_press(move |_| {
                        let saved = draft.peek().saved.clone();
                        draft.write().value = saved;
                    })
            }),
        )
        .maybe_child(
            matches!(selected, Section::Appearance | Section::Library).then(|| {
                Button::new()
                    .filled()
                    .enabled(d.loaded && d.dirty() && !d.busy)
                    .child(if d.busy { "Saving…" } else { "Save changes" })
                    .on_press(move |_| {
                        let requested = draft.peek().value.clone();
                        let previous = draft.peek().saved.clone();
                        let services = save_services.clone();
                        let runtime = runtime.clone();
                        let mut theme = theme.clone();
                        draft.write().busy = true;
                        draft.write().error = None;
                        spawn(async move {
                            let value = requested.clone();
                            let svc = services.clone();
                            let result = runtime
                                .spawn(async move {
                                    if value.folders != previous.folders {
                                        let watcher = svc
                                            .watcher
                                            .as_ref()
                                            .ok_or("Library watcher is unavailable")?;
                                        for path in value
                                            .folders
                                            .iter()
                                            .filter(|p| !previous.folders.contains(p))
                                        {
                                            watcher
                                                .add_folder(std::path::Path::new(path))
                                                .await
                                                .map_err(|e| e.to_string())?;
                                        }
                                        for path in previous
                                            .folders
                                            .iter()
                                            .filter(|p| !value.folders.contains(p))
                                        {
                                            watcher
                                                .remove_folder(std::path::Path::new(path))
                                                .await
                                                .map_err(|e| e.to_string())?;
                                        }
                                    }
                                    svc.update_config(|c| {
                                        c.theme = value.theme;
                                        c.density = value.density;
                                        c.reduce_motion = value.reduce_motion;
                                    })
                                    .map_err(|e| e.to_string())
                                })
                                .await
                                .map_err(|e| e.to_string())
                                .and_then(|r| r);
                            match result {
                                Ok(()) => match theme.set(requested.theme) {
                                    Ok(()) => draft.write().saved = requested,
                                    Err(e) => draft.write().error = Some(e.to_string()),
                                },
                                Err(e) => draft.write().error = Some(e),
                            }
                            draft.write().busy = false;
                        });
                    })
            }),
        )
}
