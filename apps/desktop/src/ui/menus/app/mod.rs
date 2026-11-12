use super::*;
use crate::ui::hooks::{PlaybackPanelView, use_library, use_playback};
use audexis_audio::{PlayerCmd, RepeatMode};
use freya_core::events::platform::{KeyboardEventName, PlatformEvent};

mod actions;
mod trigger;
pub use actions::AppAction;
use trigger::MenuTrigger;
mod dropdown;
mod model;
use crate::ui::windows::new_playlist::NewPlaylist;
pub(crate) use actions::dispatch;
use model::groups;

#[cfg(target_os = "macos")]
mod native;
#[cfg(target_os = "macos")]
use native::install_native;

#[derive(PartialEq)]
pub struct AppMenuBar;
impl Component for AppMenuBar {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let playback = use_playback();
        let shortcuts = crate::ui::keyboard::use_keyboard_settings();
        let _ = shortcuts.read();
        let router = RouterContext::try_get();
        let commands = use_consume::<crate::ui::keyboard::ShortcutCommands>();
        let library = use_library();
        let mut model = use_state(Vec::<Entry>::new);
        let busy = use_state(|| false);
        let mut open = use_state(|| None::<String>);
        let hovered = use_state(|| None::<String>);
        let edit_focus = use_state(|| None::<AccessibilityId>);
        let current = playback.state.read().clone();
        let target = MediaTarget::track(current.id, current.path.unwrap_or_default());
        let task_services = services.clone();
        let task_runtime = runtime.clone();
        let task_playback = playback.clone();
        let task_router = router.clone();
        use_future(move || {
            let services = task_services.clone();
            let runtime = task_runtime.clone();
            let playback = task_playback.clone();
            let router = task_router.clone();
            let library = library.clone();
            let commands = commands.clone();
            async move {
                let mut cache = None;
                let mut state = Err("Loading playlists…".into());
                #[cfg(target_os = "macos")]
                let mut native = None::<(muda::Menu, Vec<(muda::MenuId, Action)>)>;
                let mut installed_shortcuts = None;
                loop {
                    #[cfg(target_os = "macos")]
                    if let Some((_, actions)) = &native {
                        for event in muda::MenuEvent::receiver().try_iter() {
                            if let Some((_, action)) =
                                actions.iter().find(|(id, _)| *id == event.id)
                            {
                                if crate::ui::keyboard::is_recording() {
                                    continue;
                                }
                                if let Action::App(action) = action {
                                    if let Some(shortcut) =
                                        crate::ui::keyboard::app_shortcut(*action)
                                    {
                                        commands.0.call(shortcut);
                                        continue;
                                    }
                                }
                                let p = playback.state.peek().clone();
                                let target = MediaTarget::track(p.id, p.path.unwrap_or_default());
                                if let Err(e) = super::dispatch(
                                    *action,
                                    &target,
                                    &playback,
                                    services.clone(),
                                    runtime.clone(),
                                    router.clone(),
                                    None,
                                )
                                .await
                                {
                                    let mut error = playback.error;
                                    error.set(Some(e));
                                }
                                cache = None;
                            }
                        }
                    }
                    let p = playback.state.peek().clone();
                    let key = (p.id, library.revision());
                    if cache != Some(key) {
                        let db = services.db.clone();
                        let id = p.id;
                        state = runtime
                            .spawn(async move {
                                db.media_menu_state(&[id]).await.map_err(|e| e.to_string())
                            })
                            .await
                            .map_err(|e| e.to_string())
                            .and_then(|r| r);
                        cache = Some(key);
                    }
                    let target = MediaTarget::track(p.id, p.path.clone().unwrap_or_default());
                    let next = groups(
                        &target,
                        &state,
                        &p,
                        playback.available(),
                        *playback.panel.peek(),
                        services.watcher.is_some(),
                    );
                    let keyboard = services
                        .user_config
                        .lock()
                        .map(|c| c.keyboard.clone())
                        .unwrap_or_default();
                    if *model.peek() != next || installed_shortcuts.as_ref() != Some(&keyboard) {
                        #[cfg(target_os = "macos")]
                        if !crate::ui::windows::title_bar::windows_title_bar() {
                            match install_native(next.clone(), &keyboard) {
                                Ok(value) => native = Some(value),
                                Err(e) => {
                                    let mut error = playback.error;
                                    error.set(Some(e));
                                }
                            }
                        }
                        installed_shortcuts = Some(keyboard);
                        model.set(next);
                    }
                    let _ = runtime
                        .spawn(async {
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await
                        })
                        .await;
                }
            }
        });
        if cfg!(target_os = "macos") && !crate::ui::windows::title_bar::windows_title_bar() {
            return rect().height(Size::px(0.)).into_element();
        }
        let colors = crate::ui::theme::use_audexis_colors();
        let mut bar = rect()
            .width(Size::fill())
            .height(Size::px(44.))
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .padding((0., 8.))
            .background(colors.background)
            .border(
                Border::new()
                    .width(BorderWidth {
                        bottom: 1.0,
                        ..Default::default()
                    })
                    .fill(colors.border),
            )
            .child(
                rect()
                    .height(Size::fill())
                    .cross_align(Alignment::Center)
                    .horizontal()
                    .padding((0., 12.))
                    .window_drag()
                    .child(
                        label()
                            .text("Audexis")
                            .font_size(13.)
                            .font_weight(FontWeight::BOLD),
                    ),
            );
        for group in model.read().clone() {
            let active = open.read().as_ref() == Some(&group.title);
            let title = group.title.clone();
            let enabled = group.enabled;
            let close = EventHandler::new(move |()| open.set(None));
            let trigger = MenuTrigger {
                title,
                enabled,
                open,
                hovered,
                edit_focus,
            };
            let menu = active.then(|| {
                rect()
                    .position(Position::new_absolute().top(44.).left(0.))
                    .width(Size::px(0.))
                    .height(Size::px(0.))
                    .child(
                        Menu::new()
                            .key(group.title.clone())
                            .on_close(move |_| open.set(None))
                            .child(crate::ui::keyboard::KeyboardMenu {
                                child: rect()
                                    .content(Content::fit())
                                    .children(group.children.into_iter().map(|entry| {
                                        dropdown::item(
                                            entry,
                                            target.clone(),
                                            playback.clone(),
                                            services.clone(),
                                            runtime.clone(),
                                            router.clone(),
                                            busy,
                                            close.clone(),
                                            *edit_focus.peek(),
                                            &colors,
                                        )
                                    }))
                                    .into_element(),
                            }),
                    )
            });
            bar = bar.child(
                rect()
                    .key(group.title)
                    .height(Size::fill())
                    .main_align(Alignment::Center)
                    .child(trigger)
                    .maybe_child(menu),
            );
        }
        bar = bar.child(
            rect()
                .width(Size::flex(1.))
                .height(Size::fill())
                .window_drag(),
        );
        if crate::ui::windows::title_bar::windows_title_bar() {
            for (title, icon, action) in [
                ("Minimize", freya::icons::lucide::minus(), 0),
                ("Maximize or restore", freya::icons::lucide::square(), 1),
                ("Close", freya::icons::lucide::x(), 2),
            ] {
                bar = bar.child(
                    Button::new()
                        .flat()
                        .child(
                            rect()
                                .width(Size::px(28.))
                                .height(Size::px(24.))
                                .center()
                                .a11y_alt(title)
                                .child(
                                    SvgViewer::new((title, icon))
                                        .width(Size::px(16.))
                                        .height(Size::px(16.))
                                        .color(colors.foreground),
                                ),
                        )
                        .on_press(move |_| {
                            let platform = Platform::get();
                            let close = platform.clone();
                            platform.with_window(None, move |w| match action {
                                0 => w.set_minimized(true),
                                1 => w.set_maximized(!w.is_maximized()),
                                _ => close.close_window(w.id()),
                            });
                        }),
                );
            }
        }

        bar.into_element()
    }
}
