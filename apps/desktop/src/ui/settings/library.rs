use freya::{icons, prelude::*};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Handle;

use crate::services::AppServices;
use crate::ui::theme::use_audexis_colors;

pub struct LibraryPanel {
    pub watcher: Option<Arc<audexis_core::file_watcher::FileWatcher>>,
    pub runtime: Handle,
}

impl PartialEq for LibraryPanel {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Component for LibraryPanel {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let mut busy = use_state(|| false);
        let mut error = use_state(|| None::<String>);
        let services = use_consume::<Arc<AppServices>>();
        let folders = use_state(|| Vec::<String>::new());
        let loaded = use_state(|| false);
        let db = services.db.pool.clone();
        let load_runtime = self.runtime.clone();
        use_side_effect_with_deps(&*loaded.read(), move |is_loaded| {
            if *is_loaded {
                return;
            }
            let mut folders = folders;
            let mut loaded = loaded;
            let runtime = load_runtime.clone();
            let db = db.clone();
            spawn(async move {
                let result = runtime
                    .spawn(async move { audexis_core::file_watcher::get_library_roots(&db).await })
                    .await;
                if let Ok(Ok(values)) = result {
                    folders.set(values);
                }
                loaded.set(true);
            });
        });

        let add_watcher = self.watcher.clone();
        let rescan_watcher = self.watcher.clone();
        let runtime = self.runtime.clone();
        rect()
            .spacing(18.0)
            .child(
                label()
                    .text("Your music")
                    .font_size(25.0)
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                label()
                    .text("Manage the folders Audexis watches for music.")
                    .color(colors.muted_foreground),
            )
            .child(
                ScrollView::new()
                    .height(Size::flex(1.0))
                    .spacing(6.0)
                    .child(
                        rect()
                            .padding(18.0)
                            .spacing(12.0)
                            .background(colors.card)
                            .corner_radius(12.0)
                            .border(Border::new().width(1.0).fill(colors.border_subtle))
                            .child(
                                label()
                                    .text("Library folders")
                                    .font_weight(FontWeight::BOLD),
                            )
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(10.0)
                                    .child(
                                        Button::new()
                                            .filled()
                                            .enabled(add_watcher.is_some() && !*busy.read())
                                            .child("Add music folder")
                                            .on_press(move |_| {
                                                let watcher = add_watcher.clone();
                                                let runtime = runtime.clone();
                                                let mut folders = folders;
                                                busy.set(true);
                                                spawn(async move {
                                                    let result = match (
                                                        watcher,
                                                        rfd::AsyncFileDialog::new()
                                                            .pick_folder()
                                                            .await,
                                                    ) {
                                                        (Some(watcher), Some(folder)) => {
                                                            let path = folder.path().to_path_buf();
                                                            let displayed_path =
                                                                path.to_string_lossy().into_owned();
                                                            runtime
                                                                .spawn(async move {
                                                                    watcher
                                                                        .add_folder(&path)
                                                                        .await
                                                                        .map(|_| displayed_path)
                                                                })
                                                                .await
                                                                .map_err(|e| e.to_string())
                                                                .and_then(|result| {
                                                                    result
                                                                        .map_err(|e| e.to_string())
                                                                })
                                                        }
                                                        _ => Ok(String::new()),
                                                    };
                                                    match result {
                                                        Ok(path) if !path.is_empty() => {
                                                            let mut values = folders.write();
                                                            if !values
                                                                .iter()
                                                                .any(|value| value == &path)
                                                            {
                                                                values.push(path);
                                                                values.sort();
                                                            }
                                                            error.set(None);
                                                        }
                                                        Ok(_) => error.set(None),
                                                        Err(error_value) => {
                                                            error.set(Some(error_value))
                                                        }
                                                    }
                                                    busy.set(false);
                                                });
                                            }),
                                    )
                                    .child(
                                        Button::new()
                                            .flat()
                                            .enabled(rescan_watcher.is_some() && !*busy.read())
                                            .child("Rescan folders")
                                            .on_press(move |_| {
                                                error.set(
                                                    rescan_watcher
                                                        .as_ref()
                                                        .and_then(|watcher| watcher.rescan().err())
                                                        .map(|e| e.to_string()),
                                                );
                                            }),
                                    ),
                            )
                            .child(
                                label()
                                    .text("Changes to watched folders are applied immediately.")
                                    .font_size(12.0)
                                    .color(colors.muted_foreground),
                            )
                            .maybe_child(
                                error
                                    .read()
                                    .clone()
                                    .map(|e| label().text(e).color(colors.destructive)),
                            ),
                    )
                    .children(folders.read().iter().cloned().map(|folder| {
                        let watcher = self.watcher.clone();
                        let runtime = self.runtime.clone();
                        let folders = folders;
                        let mut error = error;
                        let folder_path = folder.clone();
                        let folder_name = PathBuf::from(folder.clone())
                            .file_name()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap_or_default()
                            .to_string();

                        rect()
                            .horizontal()
                            .content(Content::Flex)
                            .cross_align(Alignment::Center)
                            .spacing(18.0)
                            .corner_radius(12.0)
                            .padding(Gaps::new_symmetric(12.0, 18.0))
                            .background(colors.surface_subtle)
                            .border(Border::new().fill(colors.border).width(1.0))
                            .child(
                                SvgViewer::new(("folder", icons::lucide::folder()))
                                    .color(colors.accent)
                                    .width(Size::px(24.0)),
                            )
                            .child(
                                rect()
                                    .spacing(8.0)
                                    .child(label().text(folder_name).font_size(16.0))
                                    .child(
                                        label()
                                            .text(folder)
                                            .max_lines(1)
                                            .text_overflow(TextOverflow::Ellipsis)
                                            .color(colors.muted_foreground)
                                            .font_size(12.0),
                                    )
                                    .width(Size::flex(1.0)),
                            )
                            .child(
                                Button::new()
                                    .hover_background(colors.danger_soft)
                                    .border_fill(colors.transparent)
                                    .background(colors.transparent)
                                    .padding(Gaps::new_symmetric(5.0, 8.0))
                                    .child(
                                        SvgViewer::new(("folder", icons::lucide::x()))
                                            .width(Size::px(12.0)),
                                    )
                                    .enabled(watcher.is_some() && !*busy.read())
                                    .on_press(move |_| {
                                        let Some(watcher) = watcher.clone() else {
                                            return;
                                        };
                                        let runtime = runtime.clone();
                                        let folder_path = folder_path.clone();
                                        let removed_folder = folder_path.clone();
                                        let mut folders = folders;
                                        spawn(async move {
                                            match runtime
                                                .spawn(async move {
                                                    watcher
                                                        .remove_folder(std::path::Path::new(
                                                            &folder_path,
                                                        ))
                                                        .await
                                                })
                                                .await
                                            {
                                                Ok(Ok(())) => {
                                                    folders
                                                        .write()
                                                        .retain(|value| value != &removed_folder);
                                                    error.set(None);
                                                }
                                                Ok(Err(error_value)) => {
                                                    error.set(Some(error_value.to_string()))
                                                }
                                                Err(error_value) => {
                                                    error.set(Some(error_value.to_string()))
                                                }
                                            }
                                        });
                                    }),
                            )
                            .into_element()
                    })),
            )
    }
}
