use crate::ui::theme::use_audexis_colors;
use freya::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ControlSize {
    Bar,
    Mini,
    Expanded,
    Panel,
}
impl ControlSize {
    fn dimensions(self, prominent: bool) -> (f32, f32, f32) {
        match (self, prominent) {
            (Self::Bar, _) => (27., 27., 15.),
            (Self::Panel, _) => (25., 19., 13.),
            (Self::Mini, true) => (44., 44., 22.),
            (Self::Expanded, true) => (56., 56., 22.),
            _ => (32., 32., 18.),
        }
    }
}

pub(crate) fn icon_button(
    title: &'static str,
    icon: SvgViewer,
    enabled: bool,
    size: ControlSize,
    action: impl FnMut(Event<PressEventData>) + 'static,
) -> impl IntoElement {
    styled_button(title, icon, enabled, size, false, action)
}

pub(crate) fn player_button(
    title: &'static str,
    icon: impl Into<ImageSource>,
    enabled: bool,
    active: bool,
    action: impl FnMut(Event<PressEventData>) + 'static,
) -> impl IntoElement {
    player_button_sized(title, icon, enabled, active, ControlSize::Mini, action)
}

pub(crate) fn player_button_sized(
    title: &'static str,
    icon: impl Into<ImageSource>,
    enabled: bool,
    active: bool,
    size: ControlSize,
    action: impl FnMut(Event<PressEventData>) + 'static,
) -> impl IntoElement {
    let colors = use_audexis_colors();
    let color = if active {
        colors.primary
    } else if size == ControlSize::Bar {
        colors.muted_foreground
    } else {
        colors.content_secondary
    };
    icon_button(
        title,
        SvgViewer::new(icon).color(color),
        enabled,
        size,
        action,
    )
}

pub(super) fn styled_button(
    title: &'static str,
    icon: SvgViewer,
    enabled: bool,
    size: ControlSize,
    prominent: bool,
    action: impl FnMut(Event<PressEventData>) + 'static,
) -> impl IntoElement {
    let colors = use_audexis_colors();
    let (width, height, icon_size) = size.dimensions(prominent);
    TooltipContainer::new(Tooltip::new(title)).child(
        Button::new()
            .compact()
            .flat()
            .enabled(enabled)
            .theme_layout(ButtonLayoutThemePartial {
                width: Some(Preference::Specific(Size::px(width))),
                height: Some(Preference::Specific(Size::px(height))),
                padding: Some(Preference::Specific(Gaps::new_all(0.))),
                margin: Some(Preference::Specific(Gaps::new_all(0.))),
                corner_radius: prominent.then(|| Preference::Specific(CornerRadius::new_all(16.))),
            })
            .theme_colors(if prominent {
                ButtonColorsThemePartial {
                    background: Some(Preference::Specific(colors.primary)),
                    hover_background: Some(Preference::Specific(colors.accent)),
                    color: Some(Preference::Specific(colors.primary_foreground)),
                    ..Default::default()
                }
            } else {
                ButtonColorsThemePartial::default()
            })
            .child(
                rect().expanded().center().child(
                    icon.width(Size::px(icon_size))
                        .height(Size::px(icon_size))
                        .a11y_alt(title),
                ),
            )
            .on_press(action),
    )
}
