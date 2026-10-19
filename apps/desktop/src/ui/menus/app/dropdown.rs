use super::*;

pub(super) fn item(
    entry: Entry,
    target: MediaTarget,
    playback: Playback,
    services: Arc<AppServices>,
    runtime: tokio::runtime::Handle,
    router: Option<RouterContext>,
    mut busy: State<bool>,
    close: EventHandler<()>,
    edit_focus: Option<AccessibilityId>,
    colors: &crate::ui::theme::AudexisColors,
) -> Element {
    if entry.title.is_empty() {
        return rect()
            .width(Size::fill())
            .padding((4., 6.))
            .child(
                rect()
                    .height(Size::px(1.))
                    .width(Size::fill())
                    .background(colors.border_subtle),
            )
            .into_element();
    }
    let mut row = rect()
        .width(Size::px(244.))
        .height(Size::px(22.))
        .horizontal()
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(10.)
        .font_size(13.)
        .color(colors.foreground)
        .child(
            rect()
                .width(Size::px(16.))
                .height(Size::px(16.))
                .center()
                .maybe_child((entry.checked == Some(true)).then(|| {
                    SvgViewer::new(("menu-check", freya::icons::lucide::check()))
                        .width(Size::px(14.))
                        .height(Size::px(14.))
                        .color(colors.primary)
                })),
        )
        .child(
            label()
                .text(entry.title.clone())
                .width(Size::flex(1.))
                .max_lines(1),
        );
    if !entry.children.is_empty() {
        row = row.child(
            SvgViewer::new(("menu-submenu", freya::icons::lucide::chevron_right()))
                .width(Size::px(14.))
                .height(Size::px(14.))
                .color(colors.muted_foreground),
        );
    } else if let Some(shortcut) = shortcut(entry.action) {
        row = row.child(
            label()
                .text(shortcut)
                .font_size(11.)
                .color(colors.muted_foreground),
        );
    }
    if !entry.enabled {
        return rect().padding(8.).opacity(0.4).child(row).into_element();
    }
    if !entry.children.is_empty() {
        return SubMenu::new()
            .label(row)
            .children(entry.children.into_iter().map(|child| {
                item(
                    child,
                    target.clone(),
                    playback.clone(),
                    services.clone(),
                    runtime.clone(),
                    router.clone(),
                    busy,
                    close.clone(),
                    edit_focus,
                    colors,
                )
            }))
            .into_element();
    }
    MenuButton::new()
        .child(row)
        .on_press(move |_| {
            close.call(());
            if *busy.peek() {
                return;
            }
            let Some(action) = entry.action else {
                return;
            };
            if matches!(
                action,
                Action::App(
                    AppAction::Cut | AppAction::Copy | AppAction::Paste | AppAction::SelectAll
                )
            ) {
                if let Some(id) = edit_focus {
                    Platform::get().send(UserEvent::FocusAccessibilityNode(
                        AccessibilityFocusStrategy::Node(id),
                    ));
                }
            }
            busy.set(true);
            let target = target.clone();
            let playback = playback.clone();
            let services = services.clone();
            let runtime = runtime.clone();
            let router = router.clone();
            spawn(async move {
                if let Err(message) = super::super::dispatch(
                    action, &target, &playback, services, runtime, router, None,
                )
                .await
                {
                    let mut error = playback.error;
                    error.set(Some(message));
                }
                busy.set(false);
            });
        })
        .into_element()
}

fn shortcut(action: Option<Action>) -> Option<String> {
    let key = match action {
        Some(Action::App(AppAction::Cut)) => "X",
        Some(Action::App(AppAction::Copy)) => "C",
        Some(Action::App(AppAction::Paste)) => "V",
        Some(Action::App(AppAction::SelectAll)) => "A",
        _ => return None,
    };
    Some(format!(
        "{}{key}",
        if cfg!(target_os = "macos") {
            "⌘"
        } else {
            "Ctrl+"
        }
    ))
}
