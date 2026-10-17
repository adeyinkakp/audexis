use freya::{icons, prelude::*};

use crate::ui::theme::AudexisColors;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Appearance,
    Library,
    Metadata,
    CustomFields,
    Logs,
}

#[derive(PartialEq)]
pub struct Navigation {
    pub section: State<Section>,
    pub colors: AudexisColors,
}

impl Component for Navigation {
    fn render(&self) -> impl IntoElement {
        let colors = self.colors.clone();
        let section = self.section;
        rect()
            .width(Size::px(190.0))
            .height(Size::fill())
            .padding(16.0)
            .spacing(8.0)
            .background(colors.surface_subtle)
            .child(
                label()
                    .text("Settings")
                    .font_size(19.0)
                    .font_weight(FontWeight::BOLD),
            )
            .child(NavigationItem {
                label: "Appearance",
                icon: icons::lucide::palette(),
                item: Section::Appearance,
                section,
                colors: colors.clone(),
            })
            .child(NavigationItem {
                label: "Library",
                item: Section::Library,
                icon: icons::lucide::book(),
                section,
                colors: colors.clone(),
            })
            .child(NavigationItem {
                label: "Metadata",
                icon: icons::lucide::book(),
                item: Section::Metadata,
                section,
                colors: colors.clone(),
            })
            .child(NavigationItem {
                label: "Custom Fields",
                item: Section::CustomFields,
                icon: icons::lucide::settings_2(),
                section,
                colors: colors.clone(),
            })
            .child(NavigationItem {
                label: "Logs",
                icon: icons::lucide::file_warning(),
                item: Section::Logs,
                section,
                colors,
            })
    }
}

#[derive(PartialEq)]
struct NavigationItem {
    label: &'static str,
    item: Section,
    section: State<Section>,
    colors: AudexisColors,
    icon: Bytes,
}

impl Component for NavigationItem {
    fn render(&self) -> impl IntoElement {
        let selected = *self.section.read() == self.item;
        let mut section = self.section;
        let item = self.item;

        Button::new()
            .flat()
            .width(Size::fill())
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(8.0)
                    .padding(Gaps::new_symmetric(0.0, 4.0))
                    .child(
                        rect()
                            .width(Size::px(16.0))
                            .height(Size::px(16.0))
                            .cross_align(Alignment::Center)
                            .main_align(Alignment::Center)
                            .child(
                                SvgViewer::new((self.label, self.icon.clone()))
                                    .width(Size::px(14.0))
                                    .height(Size::px(14.0)),
                            ),
                    )
                    .child(self.label),
            )
            .on_press(move |_| section.set(item))
            .background(if selected {
                self.colors.primary_washed.clone()
            } else {
                self.colors.transparent.clone()
            })
            .color(if selected {
                self.colors.primary.clone()
            } else {
                self.colors.muted_foreground.clone()
            })
    }
}

#[derive(PartialEq)]
pub struct Panel {
    pub section: Section,
    pub selected: Section,
    pub child: Element,
}

impl Component for Panel {
    fn render(&self) -> impl IntoElement {
        rect()
            .width(Size::fill())
            .height(if self.section == self.selected {
                Size::fill()
            } else {
                Size::px(0.0)
            })
            .overflow(Overflow::Clip)
            .child(self.child.clone())
    }
}
