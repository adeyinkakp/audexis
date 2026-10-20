use crate::{
    services::AppServices,
    ui::theme::{RADIUS, use_audexis_colors},
};
use audexis_core::config::ServerKind;
use std::sync::Arc;

use crate::cloud_auth::{CLOUD_API_URL, login};
use freya::{icons::lucide, prelude::*};

#[derive(PartialEq)]
pub struct CloudPanel;

impl Component for CloudPanel {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let mut auto_sync = use_state(|| {
            services
                .user_config
                .lock()
                .map(|config| config.auto_sync)
                .unwrap_or(false)
        });
        let mut sync_error = use_state(|| None::<String>);
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
                                                logging_in.set(true);
                                                login_error.set(None);
                                                let (cancel, cancelled) = tokio::sync::oneshot::channel();
                                                cancel_login.set(Some(cancel));
                                                let services = cloud_services.clone();
                                                let task = runtime.spawn(async move {
                                                    let token = login(cancelled).await?;
                                                    let saved = services.save_server(ServerKind::Cloud, CLOUD_API_URL, &token).await;
                                                    if saved.is_err() { let _ = crate::cloud_auth::revoke(&token).await; }
                                                    saved
                                                });
                                                spawn(async move {
                                                    let result = task.await;
                                                    cancel_login.set(None);
                                                    match result {
                                                        Ok(Ok(connection)) => {
                                                            active_server.set(Some(connection));
                                                            custom_error.set(None);
                                                        }
                                                        Ok(Err(error)) => {
                                                            login_error.set(Some(error.to_string()))
                                                        }
                                                        Err(error) => {
                                                            login_error.set(Some(error.to_string()))
                                                        }
                                                    }
                                                    logging_in.set(false);
                                                });
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
                    .child(
                        Button::new().flat().width(Size::fill())
                            .child(
                                rect().horizontal().width(Size::fill())
                                    .cross_align(Alignment::Center).spacing(12.)
                                    .child(Checkbox::new().selected(*auto_sync.read()))
                                    .child(
                                        rect().spacing(4.)
                                            .child(label().text("Automatically sync library"))
                                            .child(label().text("Automatically upload new music and sync library changes.")
                                                .font_size(12.).color(colors.muted_foreground)),
                                    ),
                            )
                            .on_press(move |_| {
                                let enabled = !*auto_sync.peek();
                                match sync_services.update_config(|config| config.auto_sync = enabled) {
                                    Ok(()) => {
                                        auto_sync.set(enabled);
                                        sync_error.set(None);
                                    }
                                    Err(error) => sync_error.set(Some(format!("Could not save sync preference: {error}"))),
                                }
                            }),
                    )
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
