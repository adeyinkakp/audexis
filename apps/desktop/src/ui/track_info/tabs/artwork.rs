use super::super::*;
use std::time::Duration;
#[derive(PartialEq)]
pub struct Artwork;
impl Component for Artwork {
    fn render(&self) -> impl IntoElement {
        let mut session = use_consume::<State<Session>>();
        let runtime = use_consume::<Handle>();
        let s = session.read().clone();
        let images = s.values(FrameKey::AttachedPicture);
        let colors = crate::ui::theme::use_audexis_colors();
        let mut selected = use_state(|| 0usize);
        let current = (*selected.read()).min(images.len().saturating_sub(1));
        let mut body = rect()
            .width(Size::px(272.))
            .padding(12.)
            .corner_radius(16.)
            .border(Border::new().fill(colors.border).width(1.))
            .spacing(12.)
            .child(label().text(format!("Images ({})", images.len())))
            .maybe_child(s.mixed(FrameKey::AttachedPicture).then(|| {
                label().text(concat!(
                    "Artwork differs across these tracks. Editing applies the displayed artwork ",
                    "list to all selected tracks.",
                ))
            }))
            .child(
                Button::new()
                    .enabled(!s.busy)
                    .child("Add image…")
                    .on_press(move |_| {
                        let runtime = runtime.clone();
                        session.write().busy = true;
                        spawn(async move {
                            if let Some(file) = rfd::AsyncFileDialog::new()
                                .add_filter("Artwork", &["png", "jpg", "jpeg"])
                                .pick_file()
                                .await
                            {
                                let path = file.path().to_owned();
                                match runtime
                                    .spawn_blocking(move || {
                                        crate::ui::track_info::lookup::import_image(
                                            std::fs::read(path).map_err(|e| e.to_string())?,
                                        )
                                    })
                                    .await
                                {
                                    Ok(Ok(image)) => {
                                        let mut s = session.write();
                                        let mut images = s.values(FrameKey::AttachedPicture);
                                        images.push(image);
                                        s.set_values(FrameKey::AttachedPicture, images);
                                    }
                                    result => {
                                        session.write().message =
                                            format!("Could not import image: {result:?}")
                                    }
                                }
                            }
                            session.write().busy = false;
                        });
                    }),
            );
        for (index, image) in images.iter().enumerate() {
            if let TagValue::Picture {
                data, description, ..
            } = image
            {
                body = body.child(
                    Button::new()
                        .flat()
                        .theme_colors(ButtonColorsThemePartial {
                            background: Some(
                                (if current == index {
                                    colors.primary_washed
                                } else {
                                    colors.background
                                })
                                .into(),
                            ),
                            ..Default::default()
                        })
                        .child(
                            rect()
                                .horizontal()
                                .spacing(12.)
                                .child(
                                    ImageViewer::new((
                                        format!("editor-thumb-{}-{index}", s.revision),
                                        Bytes::from(data.clone()),
                                    ))
                                    .asset_age(Duration::from_secs(30))
                                    .width(Size::px(56.))
                                    .height(Size::px(56.))
                                    .aspect_ratio(AspectRatio::Max),
                                )
                                .child(
                                    label()
                                        .text(
                                            description
                                                .clone()
                                                .filter(|v| !v.is_empty())
                                                .unwrap_or_else(|| format!("Image {}", index + 1)),
                                        )
                                        .font_size(12.),
                                ),
                        )
                        .on_press(move |_| selected.set(index)),
                );
            }
        }
        let mut preview = rect().width(Size::flex(1.)).spacing(16.);
        if matches!(images.get(current), Some(TagValue::Picture { .. })) {
            preview = preview.child(PictureFields { index: current });
        } else {
            preview = preview.child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(340.))
                    .center()
                    .corner_radius(16.)
                    .background(colors.surface_subtle)
                    .border(Border::new().fill(colors.border).width(1.))
                    .child(
                        label()
                            .text("No embedded artwork")
                            .color(colors.muted_foreground),
                    ),
            );
        }
        let body = rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::Flex)
            .spacing(20.)
            .child(preview)
            .child(body);
        body
    }
}
#[derive(PartialEq)]
struct PictureFields {
    index: usize,
}
impl Component for PictureFields {
    fn render(&self) -> impl IntoElement {
        let mut session = use_consume::<State<Session>>();
        let colors = crate::ui::theme::use_audexis_colors();
        let mut description_value = use_state(String::new);
        let index = self.index;
        let s = session.read().clone();
        let images = s.values(FrameKey::AttachedPicture);
        let TagValue::Picture {
            description,
            picture_type,
            data,
            ..
        } = &images[index]
        else {
            return rect().into_element();
        };
        let text = description.clone().unwrap_or_default();
        if *description_value.peek() != text {
            description_value.set(text);
        }
        let selected_type = picture_type.unwrap_or(0);
        const TYPES: [&str; 21] = [
            "Other",
            "32×32 file icon",
            "Other file icon",
            "Front cover",
            "Back cover",
            "Leaflet",
            "Media",
            "Lead artist",
            "Artist",
            "Conductor",
            "Band / Orchestra",
            "Composer",
            "Lyricist",
            "Recording location",
            "During recording",
            "During performance",
            "Screen capture",
            "Bright coloured fish",
            "Illustration",
            "Band logo",
            "Publisher logo",
        ];
        let body = rect()
            .width(Size::fill())
            .spacing(16.)
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .spacing(16.)
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .spacing(6.)
                            .child(
                                label()
                                    .text("Type")
                                    .font_size(12.)
                                    .color(colors.muted_foreground),
                            )
                            .child(
                                Select::new()
                                    .selected_item(
                                        TYPES
                                            .get(selected_type as usize)
                                            .unwrap_or(&"Other")
                                            .to_string(),
                                    )
                                    .children(TYPES.into_iter().enumerate().map(
                                        move |(kind, title)| {
                                            MenuItem::new()
                                                .selected(kind == selected_type as usize)
                                                .child(title)
                                                .on_press(move |_| {
                                                    if session.peek().busy {
                                                        return;
                                                    }
                                                    let mut s = session.write();
                                                    let mut images =
                                                        s.values(FrameKey::AttachedPicture);
                                                    if let TagValue::Picture {
                                                        picture_type, ..
                                                    } = &mut images[index]
                                                    {
                                                        *picture_type = Some(kind as u8);
                                                    }
                                                    s.set_values(FrameKey::AttachedPicture, images);
                                                })
                                                .into_element()
                                        },
                                    )),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .spacing(6.)
                            .child(
                                label()
                                    .text("Description")
                                    .font_size(12.)
                                    .color(colors.muted_foreground),
                            )
                            .child(
                                Input::new(description_value)
                                    .enabled(!s.busy)
                                    .placeholder("Description")
                                    .on_validate(move |v: InputValidator| {
                                        let mut s = session.write();
                                        let mut images = s.values(FrameKey::AttachedPicture);
                                        if let TagValue::Picture { description, .. } =
                                            &mut images[index]
                                        {
                                            *description = Some(v.text().clone());
                                        }
                                        s.set_values(FrameKey::AttachedPicture, images);
                                    })
                                    .width(Size::fill()),
                            ),
                    ),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(340.))
                    .padding(20.)
                    .corner_radius(16.)
                    .background(colors.surface_subtle)
                    .border(Border::new().fill(colors.border).width(1.))
                    .child(
                        ImageViewer::new((
                            format!("editor-art-{}-{index}", s.revision),
                            Bytes::from(data.clone()),
                        ))
                        .asset_age(Duration::from_secs(30))
                        .width(Size::fill())
                        .height(Size::fill())
                        .aspect_ratio(AspectRatio::Max),
                    ),
            );
        let mut actions = rect().horizontal().spacing(8.);
        for (title, offset) in [("Move up", -1isize), ("Move down", 1), ("Remove", 0)] {
            actions = actions.child(
                Button::new()
                    .compact()
                    .enabled(
                        !s.busy
                            && (offset == 0
                                || (index as isize + offset >= 0
                                    && index as isize + offset < images.len() as isize)),
                    )
                    .child(title)
                    .on_press(move |_| {
                        let mut s = session.write();
                        let mut images = s.values(FrameKey::AttachedPicture);
                        if offset == 0 {
                            images.remove(index);
                        } else {
                            images.swap(index, (index as isize + offset) as usize);
                        }
                        s.set_values(FrameKey::AttachedPicture, images);
                        s.revision += 1;
                    }),
            );
        }
        body.child(actions)
            .child(
                Button::new()
                    .compact()
                    .enabled(!s.busy && index > 0)
                    .child("Make Primary")
                    .on_press(move |_| {
                        let mut s = session.write();
                        let mut images = s.values(FrameKey::AttachedPicture);
                        let image = images.remove(index);
                        images.insert(0, image);
                        s.set_values(FrameKey::AttachedPicture, images);
                        s.revision += 1;
                    }),
            )
            .into_element()
    }
}
