use super::*;

pub(super) fn auto_sync(
    sync_services: Arc<AppServices>,
    mut auto_sync: State<bool>,
    mut sync_error: State<Option<String>>,
    colors: crate::ui::theme::AudexisColors,
) -> impl IntoElement {
    Button::new()
        .flat()
        .width(Size::fill())
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .cross_align(Alignment::Center)
                .spacing(12.)
                .child(Checkbox::new().selected(*auto_sync.read()))
                .child(
                    rect()
                        .spacing(4.)
                        .child(label().text("Automatically sync library"))
                        .child(
                            label()
                                .text("Automatically upload new music and sync library changes.")
                                .font_size(12.)
                                .color(colors.muted_foreground),
                        ),
                ),
        )
        .on_press(move |_| {
            let enabled = !*auto_sync.peek();
            match sync_services.update_config(|config| config.auto_sync = enabled) {
                Ok(()) => {
                    auto_sync.set(enabled);
                    sync_error.set(None);
                }
                Err(error) => {
                    sync_error.set(Some(format!("Could not save sync preference: {error}")))
                }
            }
        })
}
