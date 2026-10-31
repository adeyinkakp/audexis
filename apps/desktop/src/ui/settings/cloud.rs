mod auth;
mod playback;
mod sync;
use crate::{
    services::AppServices,
    ui::theme::{RADIUS, use_audexis_colors},
};
use audexis_core::config::ServerKind;
use std::sync::Arc;

use crate::cloud_auth::CLOUD_API_URL;
use freya::{icons::lucide, prelude::*};

#[derive(PartialEq)]
pub struct CloudPanel;

impl Component for CloudPanel {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let sync_now_services = services.clone();
        let settings_services = services.clone();
        let quality = use_state(|| {
            services
                .user_config
                .lock()
                .map(|c| c.cloud_audio_quality)
                .unwrap_or_default()
        });
        let cache_mb = use_state(|| {
            services
                .user_config
                .lock()
                .map(|c| c.playback_cache_mb)
                .unwrap_or(2048)
        });
        let mut status = use_state(|| services.sync_status.borrow().clone());
        let status_services = services.clone();
        use_future(move || {
            let mut receiver = status_services.sync_status.subscribe();
            async move {
                while receiver.changed().await.is_ok() {
                    status.set(receiver.borrow_and_update().clone());
                }
            }
        });
        let auto_sync = use_state(|| {
            services
                .user_config
                .lock()
                .map(|config| config.auto_sync)
                .unwrap_or(false)
        });
        let sync_error = use_state(|| None::<String>);
        let sync_services = services.clone();
        let server_url = use_state(|| {
            services
                .user_config
                .lock()
                .ok()
                .and_then(|config| config.server.clone())
                .filter(|connection| connection.kind == ServerKind::Custom)
                .map(|connection| connection.url)
                .unwrap_or_default()
        });
        let mut api_key = use_state(String::new);
        let runtime = use_consume::<tokio::runtime::Handle>();
        let mut logging_in = use_state(|| false);
        let mut login_error = use_state(|| None::<String>);
        let mut cancel_login = use_state(|| None::<tokio::sync::oneshot::Sender<()>>);
        let mut active_server = use_state(|| {
            services
                .cloud
                .lock()
                .ok()
                .and_then(|cloud| cloud.as_ref().map(|details| details.connection.clone()))
        });
        let configured = services
            .user_config
            .lock()
            .ok()
            .and_then(|config| config.server.clone())
            .is_some_and(|connection| connection.credential_id.is_some());
        let mut custom_error = use_state(|| None::<String>);
        let cloud_services = services.clone();
        let custom_services = services.clone();
        let custom_runtime = runtime.clone();
        let logout_runtime = runtime.clone();
        let mut logout_error = use_state(|| None::<String>);
        let cloud_active = active_server
            .read()
            .as_ref()
            .is_some_and(|server| server.kind == ServerKind::Cloud);
        let custom_active = active_server
            .read()
            .as_ref()
            .is_some_and(|server| server.kind == ServerKind::Custom);
        let mut logout = move || {
            logging_in.set(true);
            logout_error.set(None);
            let services = services.clone();
            let task = logout_runtime.spawn(crate::cloud_auth::logout(services));
            spawn(async move {
                match task.await {
                    Ok(Ok(warning)) => {
                        active_server.set(None);
                        api_key.set(String::new());
                        login_error.set(None);
                        custom_error.set(None);
                        logout_error.set(warning);
                    }
                    Ok(Err(error)) => logout_error.set(Some(error)),
                    Err(error) => logout_error.set(Some(error.to_string())),
                }
                logging_in.set(false);
            });
        };
        let mut cloud_logout = logout.clone();

        let playback_settings = playback::playback_settings(
            settings_services,
            custom_runtime.clone(),
            quality,
            cache_mb,
            sync_error,
            colors.clone(),
        );
        ScrollView::new()
            .width(Size::fill())
            .height(Size::fill())
            .child(
                rect()
                    .width(Size::fill())
                    .spacing(24.)
                    .child(
                        rect()
                            .spacing(8.)
                            .child(
                                label()
                                    .text("Cloud")
                                    .font_size(20.)
                                    .font_weight(FontWeight::BOLD),
                            )
                            .child(
                                label()
                                    .text(
                                        "Access your music with Audexis Cloud or your own server.",
                                    )
                                    .color(colors.muted_foreground),
                            ),
                    )
                    .child(
                        rect()
                            .horizontal()
                            .width(Size::fill())
                            .content(Content::Flex)
                            .spacing(16.)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .padding(20.)
                                    .spacing(16.)
                                    .corner_radius(RADIUS)
                                    .background(colors.card)
                                    .border(Border::new().width(1.).fill(colors.border))
                                    .child(
                                        SvgViewer::new(lucide::cloud())
                                            .width(Size::px(24.))
                                            .height(Size::px(24.))
                                            .stroke(colors.primary),
                                    )
                                    .child(
                                        label()
                                            .text("Audexis Cloud")
                                            .font_size(16.)
                                            .font_weight(FontWeight::BOLD),
                                    )
                                    .child(
                                        label()
                                            .text("Sign in to access your cloud music library.")
                                            .font_size(13.)
                                            .color(colors.muted_foreground),
                                    )
                                    .child(
                                        Button::new()
                                            .filled()
                                            .enabled(!*logging_in.read() && !custom_active)
                                            .child(if cloud_active {
                                                "Log out"
                                            } else if *logging_in.read() {
                                                "Waiting for login…"
                                            } else {
                                                "Log in to Audexis Cloud"
                                            })
                                            .on_press(move |_| {
                                                if cloud_active {
                                                    cloud_logout();
                                                    return;
                                                }
                                                auth::login(cloud_services.clone(), runtime.clone(), logging_in, login_error, cancel_login, active_server, custom_error);
                                            }),
                                    )
                                    .maybe_child(cancel_login.read().is_some().then(|| {
                                        rect().spacing(8.)
                                            .child(label().text("Complete sign-in in your browser. This request expires after 2 minutes.")
                                                .font_size(12.).color(colors.muted_foreground))
                                            .child(Button::new().outline().child("Cancel sign-in").on_press(move |_| {
                                                if let Some(cancel) = cancel_login.write().take() {
                                                    let _ = cancel.send(());
                                                }
                                            }))
                                    }))
                                    .maybe_child(active_server.read().as_ref().is_some_and(|server| server.kind == ServerKind::Cloud).then(|| {
                                        label().text("Audexis Cloud is active.").font_size(12.)
                                    }))
                                    .maybe_child(login_error.read().clone().map(|error| {
                                        label().text(error).font_size(12.).color(colors.destructive)
                                    })),
                            )
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .padding(20.)
                                    .spacing(16.)
                                    .corner_radius(RADIUS)
                                    .background(colors.card)
                                    .border(Border::new().width(1.).fill(colors.border))
                                    .child(
                                        SvgViewer::new(lucide::server())
                                            .width(Size::px(24.))
                                            .height(Size::px(24.))
                                            .stroke(colors.primary),
                                    )
                                    .child(
                                        label()
                                            .text("Custom Server")
                                            .font_size(16.)
                                            .font_weight(FontWeight::BOLD),
                                    )
                                    .child(
                                        label()
                                            .text("Use your own Audexis server.")
                                            .font_size(13.)
                                            .color(colors.muted_foreground),
                                    )
                                    .child(
                                        rect()
                                            .width(Size::fill())
                                            .spacing(6.)
                                            .child(label().text("Server URL").font_size(13.))
                                            .child(
                                                Input::new(server_url)
                                                    .width(Size::fill())
                                                    .placeholder("https://music.example.com"),
                                            ),
                                    )
                                    .child(
                                        rect()
                                            .width(Size::fill())
                                            .spacing(6.)
                                            .child(label().text("API key").font_size(13.))
                                            .child(
                                                Input::new(api_key)
                                                    .width(Size::fill())
                                                    .placeholder("Enter your API key")
                                                    .mode(InputMode::new_password()),
                                            ),
                                    )
                                    .child(
                                        Button::new().outline()
                                            .enabled(!*logging_in.read() && !cloud_active)
                                            .child(if custom_active { "Log out" } else { "Save custom server" })
                                            .on_press(move |_| {
                                                if custom_active {
                                                    logout();
                                                    return;
                                                }
                                                let url = server_url.peek().clone();
                                                let key = api_key.peek().clone();
                                                let services = custom_services.clone();
                                                logging_in.set(true);
                                                custom_error.set(None);
                                                let task = custom_runtime.spawn(async move {
                                                    services.save_server(ServerKind::Custom, &url, &key).await
                                                });
                                                spawn(async move {
                                                    match task.await {
                                                        Ok(Ok(connection)) => {
                                                            active_server.set(Some(connection));
                                                            api_key.set(String::new());
                                                            login_error.set(None);
                                                        }
                                                        Ok(Err(error)) => custom_error.set(Some(error)),
                                                        Err(error) => custom_error.set(Some(error.to_string())),
                                                    }
                                                    logging_in.set(false);
                                                });
                                            }),
                                    )
                                    .maybe_child(active_server.read().as_ref()
                                        .filter(|server| server.kind == ServerKind::Custom)
                                        .map(|server| label().text(format!("Active server: {}", server.url)).font_size(12.)))
                                    .maybe_child(custom_error.read().clone().map(|error| {
                                        label().text(error).font_size(12.).color(colors.destructive)
                                    })),
                            ),
                    )
                    .child(sync::auto_sync(sync_services, auto_sync, sync_error, colors.clone()))
                    .child(playback_settings)
                    .child(label().text(match &*status.read() {
                        audexis_core::cloud_sync::CloudSyncStatus::Running=>"Syncing your library…".into(),
                        audexis_core::cloud_sync::CloudSyncStatus::Failed(e)=>format!("Sync needs attention: {e}"),
                        audexis_core::cloud_sync::CloudSyncStatus::Stopped=>"Sign in to start syncing.".into(),
                        audexis_core::cloud_sync::CloudSyncStatus::Idle=>"Music and playlists are available in your library. Manage sync issues in Cloud Sync.".into(),
                    }).font_size(12.).color(colors.muted_foreground))
                    .maybe_child(active_server.read().is_some().then(||Button::new().outline().child("Sync now").on_press(move |_|sync_now_services.sync_now())))
                    .maybe_child(sync_error.read().clone().map(|error| {
                        label().text(error).font_size(12.).color(colors.destructive)
                    }))
                    .maybe_child(active_server.read().is_some().then(|| {
                        label().text("Log out before switching providers.").font_size(12.).color(colors.muted_foreground)
                    }))
                    .maybe_child(active_server.read().as_ref()
                        .filter(|server| server.credential_id.is_none())
                        .map(|_| label()
                            .text("Keyring unavailable. Signed in for this session only; you’ll need to sign in again after closing Audexis.")
                            .font_size(12.).color(colors.muted_foreground)))
                    .maybe_child(logout_error.read().clone().map(|error| {
                        label().text(error).font_size(12.).color(colors.destructive)
                    }))
                    .maybe_child((configured && active_server.read().is_none()).then(|| {
                        label().text("Saved credentials could not be loaded. Log in again or re-enter your API key.")
                            .font_size(12.).color(colors.destructive)
                    })),
            )
    }
}
