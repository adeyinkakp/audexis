use crate::{
    services::AppServices,
    ui::{components::SmoothScroll, hooks::use_library, theme::use_audexis_colors},
};
use freya::prelude::*;
use std::sync::Arc;
#[derive(PartialEq)]
pub struct Missing;
impl Component for Missing {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let library = use_library();
        let mut error = use_state(|| None::<String>);
        let mut busy = use_state(|| false);
        let mut retry = use_state(|| 0u64);
        let key = (library.revision(), *retry.read());
        let db = services.db.clone();
        let load_runtime = runtime.clone();
        let mut loader = use_hook(move || {
            FutureTask::create(move || {
                let db = db.clone();
                let runtime = load_runtime.clone();
                async move {
                    runtime
                        .spawn(async move {
                            audexis_core::database::missing::get_missing_files(&db).await
                        })
                        .await
                        .map_err(|e| e.to_string())?
                }
            })
        });
        use_side_effect_with_deps(&key, move |_| {
            loader.cancel();
            loader.start();
        });
        let state = loader.state();
        let result = state.try_as_fulfilled();
        let mut body = rect()
            .width(Size::fill())
            .padding(24.)
            .spacing(20.)
            .child(
                label()
                    .text("Missing Files")
                    .font_size(24.)
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                label()
                    .text(concat!(
                        "These tracks are no longer at their saved locations. Relink a track to ",
                        "preserve its favorites, playlists, and listening history, or delete its ",
                        "library record permanently.",
                    ))
                    .font_size(14.)
                    .color(colors.muted_foreground),
            )
            .maybe_child(
                error
                    .read()
                    .clone()
                    .map(|e| label().text(e).color(colors.destructive)),
            );
        match result {
            None => body = body.child(label().text("Loading missing files…")),
            Some(Err(e)) => {
                body = body.child(
                    Button::new()
                        .child(format!("{e} · Try again"))
                        .on_press(move |_| *retry.write() += 1),
                )
            }
            Some(Ok(files)) => {
                body = body.child(
                    label()
                        .text(format!("{} missing files", files.len()))
                        .font_size(12.)
                        .color(colors.muted_foreground),
                );
                if files.is_empty() {
                    body = body.child(
                        rect()
                            .width(Size::fill())
                            .padding(40.)
                            .center()
                            .corner_radius(16.)
                            .background(colors.surface_subtle)
                            .border(Border::new().width(1.).fill(colors.border))
                            .child(label().text("No missing files"))
                            .child(
                                label()
                                    .text("Every indexed track is currently available.")
                                    .font_size(12.)
                                    .color(colors.muted_foreground),
                            ),
                    );
                }
                for file in files {
                    let id = file.id;
                    let mut row = rect()
                        .width(Size::fill())
                        .horizontal()
                        .content(Content::Flex)
                        .spacing(16.)
                        .padding(16.)
                        .corner_radius(16.)
                        .background(colors.surface_overlay)
                        .border(Border::new().width(1.).fill(colors.border))
                        .child(
                            rect()
                                .width(Size::flex(1.))
                                .spacing(6.)
                                .child(label().text(crate::ui::display_text(
                                    if file.title.is_empty() {
                                        file.file_name.clone()
                                    } else {
                                        file.title.clone()
                                    },
                                )))
                                .child(
                                    label()
                                        .text(crate::ui::display_text(file.artist.clone()))
                                        .font_size(12.)
                                        .color(colors.muted_foreground),
                                )
                                .child(
                                    label()
                                        .text(file.path.clone())
                                        .font_size(11.)
                                        .color(colors.content_faint),
                                ),
                        );
                    for delete in [false, true] {
                        let services = services.clone();
                        let runtime = runtime.clone();
                        row = row.child(
                            Button::new()
                                .enabled(!*busy.read())
                                .child(if delete { "Delete record" } else { "Relink" })
                                .on_press(move |_| {
                                    let services = services.clone();
                                    let runtime = runtime.clone();
                                    busy.set(true);
                                    error.set(None);
                                    spawn(async move {
                                        if delete {
                                            let description = concat!(
                                                "This removes the library record and its ",
                                                "playlist and listening history entries. ",
                                                "It does not delete an audio file."
                                            );
                                            let confirm = rfd::AsyncMessageDialog::new()
                                                .set_title("Delete missing file record?")
                                                .set_description(description)
                                                .set_buttons(rfd::MessageButtons::YesNo)
                                                .show()
                                                .await;
                                            if confirm == rfd::MessageDialogResult::Yes {
                                                let result = runtime
                                                    .spawn(async move {
                                                        audexis_core::database::missing::delete_missing_file(
                                                            &services.library_files,
                                                            &services.db,
                                                            id,
                                                        )
                                                        .await
                                                    })
                                                    .await;
                                                if let Err(e) = result.map_err(|e| e.to_string()).and_then(|r| r) {
                                                    error.set(Some(e));
                                                }
                                            }
                                        } else if let Some(file) = rfd::AsyncFileDialog::new()
                                            .set_title("Locate the audio file")
                                            .add_filter("Audio", &["mp3", "flac", "ogg", "opus", "m4a", "mp4", "wav"])
                                            .pick_file()
                                            .await
                                        {
                                            let path = file.path().to_owned();
                                            let result = runtime
                                                .spawn(async move {
                                                    audexis_core::database::missing::relink_missing_file(
                                                        &services.library_files,
                                                        &services.db,
                                                        id,
                                                        path,
                                                    )
                                                    .await
                                                })
                                                .await;
                                            if let Err(e) = result.map_err(|e| e.to_string()).and_then(|r| r) {
                                                error.set(Some(e));
                                            }
                                        }
                                        busy.set(false);
                                        *retry.write() += 1;
                                    });
                                }),
                        );
                    }
                    body = body.child(row);
                }
            }
        }
        SmoothScroll::new().child(body.child(rect().height(Size::px(80.))))
    }
}
