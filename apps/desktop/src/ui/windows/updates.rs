use crate::{
    services::AppServices,
    updater::{self, Phase, Status},
};
use freya::prelude::*;
use std::sync::Arc;
use tokio::runtime::Handle;

pub fn use_update_checks(services: Arc<AppServices>, runtime: Handle) {
    let start_services = services.clone();
    let start_runtime = runtime.clone();
    use_hook(move || updater::start(start_services, &start_runtime));
    use_future(move || {
        let services = services.clone();
        let runtime = runtime.clone();
        let mut updates = updater::updates().status.subscribe();
        async move {
            loop {
                let phase = updates.borrow_and_update().phase.clone();
                match phase {
                    Phase::Ready => open(services.clone(), runtime.clone()),
                    Phase::Installed => {
                        drop(Platform::get().post_callback(|_, context| context.exit()));
                        break;
                    }
                    _ => {}
                }
                if updates.changed().await.is_err() {
                    break;
                }
            }
        }
    });
}

pub fn open(services: Arc<AppServices>, runtime: Handle) {
    drop(Platform::get().post_callback(move |_, context| {
        let manager = super::manager::window_manager();
        if let Some(id) = manager.update_window()
            && let Some(window) = context.windows.get(&id)
        {
            window.window().set_minimized(false);
            window.window().focus_window();
            return;
        }
        let on_close = manager.clone();
        let config =
            super::title_bar::configure(WindowConfig::new_app(UpdateWindow { services, runtime }))
                .with_title("Audexis Update")
                .with_size(480., 360.)
                .with_min_size(480., 360.)
                .with_resizable(false)
                .with_on_close(move |_, _| {
                    if matches!(
                        updater::updates().status.borrow().phase,
                        Phase::Downloading | Phase::Installed
                    ) {
                        return CloseDecision::KeepOpen;
                    }
                    on_close.set_update_window(None);
                    CloseDecision::Close
                });
        manager.set_update_window(Some(context.launch_window(config)));
    }));
}

struct UpdateWindow {
    services: Arc<AppServices>,
    runtime: Handle,
}

impl App for UpdateWindow {
    fn render(&self) -> impl IntoElement {
        let services = self.services.clone();
        let runtime = self.runtime.clone();
        use_provide_context(move || services);
        use_provide_context(move || runtime);
        crate::ui::theme::use_app_theme(self.services.clone());
        rect()
            .expanded()
            .theme_background()
            .theme_color()
            .content(Content::Flex)
            .maybe_child(
                cfg!(target_os = "macos").then(|| super::title_bar::TitleBar {
                    child: label().text("").into_element(),
                }),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .child(UpdatePanel),
            )
    }
}

pub fn use_update_status() -> State<Status> {
    let mut status = use_state(|| updater::updates().status.borrow().clone());
    use_future(move || {
        let mut updates = updater::updates().status.subscribe();
        async move {
            loop {
                status.set(updates.borrow_and_update().clone());
                if updates.changed().await.is_err() {
                    break;
                }
            }
        }
    });
    status
}

#[derive(PartialEq)]
struct UpdatePanel;
impl Component for UpdatePanel {
    fn render(&self) -> impl IntoElement {
        let runtime = use_consume::<Handle>();
        let status = use_update_status().read().clone();
        let colors = crate::ui::theme::use_audexis_colors();
        let busy = matches!(
            status.phase,
            Phase::Checking | Phase::Downloading | Phase::Installed
        );
        let title = if let Some(update) = &status.update {
            format!("Audexis {}", update.version)
        } else {
            "Software Update".into()
        };
        let message = match &status.phase {
            Phase::Idle => format!("Audexis {} is up to date.", env!("CARGO_PKG_VERSION")),
            Phase::Checking => "Checking for updates…".into(),
            Phase::Ready => {
                "A new version is available. Audexis will restart after installing it.".into()
            }
            Phase::Downloading => "Downloading and installing the update…".into(),
            Phase::Installed => "Restarting Audexis…".into(),
            Phase::Error(error) => format!("Could not update Audexis: {error}"),
        };
        let update = status.update.clone();
        rect()
            .expanded()
            .theme_background()
            .theme_color()
            .content(Content::Flex)
            .padding(24.)
            .spacing(20.)
            .child(
                label()
                    .text(title)
                    .font_size(22.)
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                ScrollView::new()
                    .height(Size::flex(1.))
                    .width(Size::fill())
                    .child(label().text(message).color(colors.muted_foreground)),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(40.))
                    .horizontal()
                    .spacing(12.)
                    .main_align(Alignment::End)
                    .child(
                        Button::new()
                            .flat()
                            .enabled(!busy)
                            .child("Later")
                            .on_press(|_| {
                                let platform = Platform::get();
                                let close = platform.clone();
                                platform.with_window(None, move |window| {
                                    close.close_window(window.id())
                                });
                            }),
                    )
                    .maybe_child(update.is_some().then(|| {
                        Button::new()
                            .filled()
                            .enabled(!busy)
                            .child("Install update")
                            .on_press(move |_| {
                                if let Some(update) = &update {
                                    runtime.spawn(updater::updates().install(update.clone()));
                                }
                            })
                    })),
            )
    }
}
