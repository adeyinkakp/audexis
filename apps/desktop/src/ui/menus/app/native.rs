use super::*;

#[cfg(target_os = "macos")]
pub(super) fn install_native(
    groups: Vec<Entry>,
) -> Result<(muda::Menu, Vec<(muda::MenuId, Action)>), String> {
    use muda::{Menu, PredefinedMenuItem as P, Submenu};
    let menu = Menu::new();
    let mut actions = vec![];
    let app = Submenu::new("Audexis", true);
    app.append(&P::about(Some("About Audexis"), None))
        .map_err(|e| e.to_string())?;
    app.append(
        native_item(
            super::model::app("Settings…", AppAction::Settings),
            &mut actions,
        )?
        .as_ref(),
    )
    .map_err(|e| e.to_string())?;
    app.append_items(&[
        &P::separator(),
        &P::services(None),
        &P::separator(),
        &P::hide(None),
        &P::hide_others(None),
        &P::show_all(None),
        &P::separator(),
        &P::quit(None),
    ])
    .map_err(|e| e.to_string())?;
    menu.append(&app).map_err(|e| e.to_string())?;
    for group in groups {
        if group.title == "Help" {
            let window = Submenu::new("Window", true);
            window
                .append_items(&[
                    &P::minimize(None),
                    &P::maximize(None),
                    &P::close_window(None),
                ])
                .map_err(|e| e.to_string())?;
            menu.append(&window).map_err(|e| e.to_string())?;
        }
        menu.append(native_item(group, &mut actions)?.as_ref())
            .map_err(|e| e.to_string())?;
    }
    menu.init_for_nsapp();
    Ok((menu, actions))
}
