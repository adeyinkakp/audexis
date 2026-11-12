use crate::ui::theme::use_audexis_colors;
use freya::{icons::lucide, prelude::*};
use std::sync::Arc;
pub struct LibraryPanel {
    pub watcher: Option<Arc<audexis_core::file_watcher::FileWatcher>>,
}
impl PartialEq for LibraryPanel {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Component for LibraryPanel {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let mut draft = use_consume::<State<super::Draft>>();
        let mut choosing = use_state(|| false);
        let d = draft.read().clone();
        let watcher = self.watcher.clone();
        let mut body = rect()
            .width(Size::fill())
            .spacing(20.)
            .child(
                rect()
                    .spacing(8.)
                    .child(
                        label()
                            .text("Your music")
                            .font_size(20.)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        label()
                            .text("Manage the folders Audexis watches for music.")
                            .color(colors.muted_foreground),
                    ),
            )
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .child(
                        label()
                            .text("Library folders")
                            .font_weight(FontWeight::BOLD)
                            .width(Size::flex(1.)),
                    )
                    .child(
                        Button::new()
                            .outline()
                            .enabled(d.loaded && !d.busy && !*choosing.read())
                            .child("+ Add folder")
                            .on_press(move |_| {
                                choosing.set(true);
                                spawn(async move {
                                    if let Some(folder) =
                                        rfd::AsyncFileDialog::new().pick_folder().await
                                    {
                                        let path = folder.path().to_string_lossy().to_string();
                                        let mut d = draft.write();
                                        if !d.value.folders.contains(&path) {
                                            d.value.folders.push(path);
                                            d.value.folders.sort();
                                        }
                                    }
                                    choosing.set(false);
                                });
                            }),
                    ),
            );
        for path in &d.value.folders {
            let remove = path.clone();
            body = body.child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .spacing(12.)
                    .padding(14.)
                    .corner_radius(12.)
                    .background(colors.surface_subtle)
                    .border(Border::new().width(1.).fill(colors.border))
                    .child(
                        SvgViewer::new(lucide::folder())
                            .width(Size::px(20.))
                            .height(Size::px(20.))
                            .stroke(colors.primary),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .spacing(4.)
                            .child(
                                label().text(
                                    std::path::Path::new(path)
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                ),
                            )
                            .child(
                                label()
                                    .text(path.clone())
                                    .font_size(12.)
                                    .color(colors.muted_foreground),
                            ),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .compact()
                            .enabled(!d.busy)
                            .child(
                                SvgViewer::new(lucide::x())
                                    .width(Size::px(14.))
                                    .height(Size::px(14.)),
                            )
                            .on_press(move |_| {
                                draft.write().value.folders.retain(|p| p != &remove)
                            }),
                    ),
            );
        }
        if d.value.folders.is_empty() {
            body = body.child(
                label()
                    .text(if d.loaded {
                        "No folders added yet."
                    } else {
                        "Loading folders…"
                    })
                    .color(colors.muted_foreground),
            );
        }
        body = body
            .child(
                label()
                    .text(concat!(
                        "Removing a folder removes its songs from the library and playlists when ",
                        "you save. Files aren't removed from disk.",
                    ))
                    .font_size(12.)
                    .color(colors.muted_foreground),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(1.))
                    .background(colors.border),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::Flex)
                    .spacing(16.)
                    .cross_align(Alignment::Center)
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .spacing(6.)
                            .child(label().text("Refresh library"))
                            .child(
                                label()
                                    .text("Scan saved folders for changes and missing metadata.")
                                    .font_size(12.)
                                    .color(colors.muted_foreground),
                            ),
                    )
                    .child(
                        Button::new()
                            .outline()
                            .enabled(
                                watcher.is_some()
                                    && !d.busy
                                    && !d.value.folders.is_empty()
                                    && d.value.folders == d.saved.folders,
                            )
                            .child("Rescan")
                            .on_press(move |_| {
                                draft.write().error = watcher
                                    .as_ref()
                                    .and_then(|w| w.rescan().err())
                                    .map(|e| e.to_string());
                            }),
                    ),
            )
            .maybe_child((d.value.folders != d.saved.folders).then(|| {
                label()
                    .text("Save your folder changes before rescanning.")
                    .font_size(12.)
                    .color(colors.muted_foreground)
            }));
        ScrollView::new()
            .width(Size::fill())
            .height(Size::fill())
            .child(body)
    }
}
