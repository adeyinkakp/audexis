use super::*;

pub fn audexis_light_theme() -> Theme {
    audexis_theme(ThemeMode::Light)
}

pub fn audexis_dark_theme() -> Theme {
    audexis_theme(ThemeMode::Dark)
}

pub fn audexis_theme(mode: ThemeMode) -> Theme {
    let p = palette(mode);
    let mut theme = match mode {
        ThemeMode::Light => light_theme(),
        ThemeMode::Dark => dark_theme(),
    };
    theme.name = match mode {
        ThemeMode::Light => "audexis-light",
        ThemeMode::Dark => "audexis-dark",
    };

    let c = &mut theme.colors;
    c.primary = p.primary;
    c.secondary = p.secondary;
    c.tertiary = p.accent;
    c.warning = p.warning;
    c.error = p.destructive;
    c.background = p.background;
    c.surface_primary = p.muted;
    c.surface_secondary = p.hover;
    c.surface_tertiary = p.card;
    c.surface_inverse = p.scrollbar_thumb;
    c.surface_inverse_secondary = p.scrollbar_thumb;
    c.surface_inverse_tertiary = p.scrollbar_thumb;
    c.border = p.border;
    c.border_focus = p.ring;
    c.border_disabled = p.border_subtle;
    c.text_primary = p.foreground;
    c.text_secondary = p.muted_foreground;
    c.text_placeholder = p.muted_foreground;
    c.text_inverse = p.primary_foreground;
    c.text_highlight = p.accent;
    c.focus = p.primary_washed;
    c.active = p.active;
    c.disabled = p.muted;
    c.overlay = p.shade;
    c.shadow = p.shade;

    theme.set(
        "scrollbar",
        ScrollBarThemePreference {
            background: Preference::Specific(p.scrollbar_track),
            thumb_background: Preference::Specific(p.scrollbar_thumb),
            hover_thumb_background: Preference::Specific(p.scrollbar_thumb),
            active_thumb_background: Preference::Specific(p.scrollbar_thumb),
            size: Preference::Specific(10.0),
        },
    );
    theme.set(
        "filled_button",
        ButtonColorsThemePreference {
            background: Preference::Specific(p.primary),
            hover_background: Preference::Specific(p.accent),
            border_fill: Preference::Specific(Color::TRANSPARENT),
            focus_border_fill: Preference::Specific(p.ring),
            color: Preference::Specific(p.primary_foreground),
        },
    );
    for (key, vertical, horizontal) in [
        ("button_layout", 6.0, 12.0),
        ("compact_button_layout", 3.0, 6.0),
        ("expanded_button_layout", 10.0, 16.0),
    ] {
        theme.set(
            key,
            ButtonLayoutThemePreference {
                padding: Preference::Specific(Gaps::new(
                    vertical, horizontal, vertical, horizontal,
                )),
                margin: Preference::Specific(Gaps::new_all(0.0)),
                corner_radius: Preference::Specific(CornerRadius::new_all(RADIUS)),
                width: Preference::Specific(Size::Inner),
                height: Preference::Specific(Size::Inner),
            },
        );
    }
    for (key, padding) in [
        ("input_layout", 8.0),
        ("compact_input_layout", 4.0),
        ("expanded_input_layout", 12.0),
    ] {
        theme.set(
            key,
            InputLayoutThemePreference {
                corner_radius: Preference::Specific(CornerRadius::new_all(RADIUS)),
                inner_margin: Preference::Specific(Gaps::new_all(padding)),
            },
        );
    }
    theme.set(
        "menu_container",
        MenuContainerThemePreference {
            background: Preference::Specific(p.card),
            padding: Preference::Specific(Gaps::new_all(6.)),
            shadow: Preference::Specific(p.shade),
            border_fill: Preference::Specific(p.border_subtle),
            corner_radius: Preference::Specific(CornerRadius::new_all(10.)),
        },
    );
    theme.set(
        "menu_item",
        MenuItemThemePreference {
            background: Preference::Specific(Color::TRANSPARENT),
            hover_background: Preference::Specific(p.hover),
            select_background: Preference::Specific(p.primary_washed),
            border_fill: Preference::Specific(Color::TRANSPARENT),
            select_border_fill: Preference::Specific(p.primary_border),
            corner_radius: Preference::Specific(CornerRadius::new_all(5.)),
            color: Preference::Specific(p.foreground),
        },
    );
    theme.set(PALETTE_KEY, p);
    theme
}
