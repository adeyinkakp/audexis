use crate::ui::theme::use_audexis_colors;
use audexis_core::config::{Density, ThemePreference};
use freya::{icons::lucide, prelude::*};
#[derive(PartialEq)]
pub struct AppearancePanel;
impl Component for AppearancePanel {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let mut draft = use_consume::<State<super::Draft>>();
        let d = draft.read().clone();
        let mut cards = rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::Flex)
            .spacing(12.);
        for (value, title, icon) in [
            (ThemePreference::Light, "Light", lucide::sun()),
            (ThemePreference::Dark, "Dark", lucide::moon()),
            (ThemePreference::System, "System", lucide::monitor()),
        ] {
            let active = d.value.theme == value;
            let preview: Fill = match value {
                ThemePreference::Dark => Color::from_rgb(24, 24, 27).into(),
                ThemePreference::Light => Color::from_rgb(245, 245, 244).into(),
                ThemePreference::System => LinearGradient::new()
                    .angle(90.)
                    .stop((Color::from_rgb(245, 245, 244), 0.))
                    .stop((Color::from_rgb(245, 245, 244), 50.))
                    .stop((Color::from_rgb(24, 24, 27), 50.))
                    .stop((Color::from_rgb(24, 24, 27), 100.))
                    .into(),
            };
            cards = cards.child(
                rect().width(Size::flex(1.)).child(
                    Button::new()
                        .flat()
                        .width(Size::fill())
                        .enabled(!d.busy)
                        .theme_colors(ButtonColorsThemePartial {
                            background: Some(
                                (if active {
                                    colors.primary_washed
                                } else {
                                    colors.background
                                })
                                .into(),
                            ),
                            border_fill: Some(
                                (if active {
                                    colors.primary
                                } else {
                                    colors.border
                                })
                                .into(),
                            ),
                            ..Default::default()
                        })
                        .child(
                            rect()
                                .width(Size::fill())
                                .spacing(12.)
                                .child(
                                    rect()
                                        .width(Size::fill())
                                        .height(Size::px(64.))
                                        .horizontal()
                                        .content(Content::Flex)
                                        .spacing(8.)
                                        .padding(8.)
                                        .corner_radius(8.)
                                        .background(preview)
                                        .child(
                                            rect()
                                                .width(Size::px(16.))
                                                .height(Size::fill())
                                                .corner_radius(3.)
                                                .background(colors.primary_soft),
                                        )
                                        .child(
                                            rect()
                                                .width(Size::flex(1.))
                                                .spacing(6.)
                                                .padding((4., 0.))
                                                .child(
                                                    rect()
                                                        .width(Size::percent(67.))
                                                        .height(Size::px(6.))
                                                        .corner_radius(3.)
                                                        .background(colors.primary),
                                                )
                                                .children([100., 100., 75.].into_iter().map(|w| {
                                                    rect()
                                                        .width(Size::percent(w))
                                                        .height(Size::px(4.))
                                                        .corner_radius(2.)
                                                        .background(colors.content_faint)
                                                        .into_element()
                                                })),
                                        ),
                                )
                                .child(
                                    rect()
                                        .horizontal()
                                        .width(Size::fill())
                                        .content(Content::Flex)
                                        .spacing(6.)
                                        .cross_align(Alignment::Center)
                                        .child(
                                            SvgViewer::new(icon)
                                                .width(Size::px(13.))
                                                .height(Size::px(13.))
                                                .stroke(colors.foreground),
                                        )
                                        .child(
                                            label()
                                                .text(title)
                                                .font_size(12.)
                                                .width(Size::flex(1.)),
                                        )
                                        .maybe_child(active.then(|| {
                                            SvgViewer::new(lucide::check())
                                                .width(Size::px(13.))
                                                .height(Size::px(13.))
                                                .stroke(colors.primary)
                                        })),
                                ),
                        )
                        .on_press(move |_| draft.write().value.theme = value),
                ),
            );
        }
        let density_options = [
            (Density::Compact, "Compact"),
            (Density::Default, "Balanced"),
            (Density::Comfort, "Comfortable"),
        ]
        .into_iter()
        .map(move |(value, title)| {
            rect()
                .width(Size::flex(1.))
                .child(
                    Button::new()
                        .flat()
                        .width(Size::fill())
                        .enabled(!d.busy)
                        .theme_colors(ButtonColorsThemePartial {
                            background: Some(
                                (if value == d.value.density {
                                    colors.background
                                } else {
                                    colors.surface_subtle
                                })
                                .into(),
                            ),
                            ..Default::default()
                        })
                        .child(label().text(title).font_size(12.))
                        .on_press(move |_| draft.write().value.density = value),
                )
                .into_element()
        });
        let density = rect()
            .width(Size::fill())
            .spacing(12.)
            .child(
                label()
                    .text("Song row spacing")
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::Flex)
                    .padding(4.)
                    .corner_radius(12.)
                    .background(colors.surface_subtle)
                    .border(Border::new().width(1.).fill(colors.border))
                    .children(density_options),
            );
        let reduce_motion = rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::Flex)
            .spacing(24.)
            .padding(16.)
            .corner_radius(12.)
            .border(Border::new().width(1.).fill(colors.border))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .spacing(6.)
                    .child(label().text("Reduce motion").font_weight(FontWeight::BOLD))
                    .child(
                        label()
                            .text(concat!(
                                "Minimizes or eliminates non-essential animations, screen movements, and ",
                                "transition effects"
                            ))
                            .font_size(12.)
                            .color(colors.muted_foreground),
                    ),
            )
            .child(
                Button::new()
                    .flat()
                    .compact()
                    .enabled(!d.busy)
                    .child(Checkbox::new().selected(d.value.reduce_motion))
                    .on_press(move |_| {
                        let value = draft.peek().value.reduce_motion;
                        draft.write().value.reduce_motion = !value;
                    }),
            );
        let content = rect()
            .width(Size::fill())
            .spacing(24.)
            .child(
                rect()
                    .spacing(8.)
                    .child(
                        label()
                            .text("Make yourself at home")
                            .font_size(20.)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        label()
                            .text("Change how Audexis looks.")
                            .color(colors.muted_foreground),
                    ),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .spacing(12.)
                    .child(label().text("Appearance").font_weight(FontWeight::BOLD))
                    .child(cards)
                    .child(
                        label()
                            .text("System follows your device’s light and dark appearance.")
                            .font_size(12.)
                            .color(colors.muted_foreground),
                    ),
            )
            .child(density)
            .child(reduce_motion);
        ScrollView::new()
            .width(Size::fill())
            .height(Size::fill())
            .child(content)
    }
}
