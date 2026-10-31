use super::{controls::PlayerControls, details::PlayerDetails, header::PlayerHeader};
use crate::ui::{
    components::{ArtworkBackdrop, ExpandedPlaybackPanel, VisibilityViewport},
    hooks::use_playback,
    theme::use_audexis_colors,
};
use freya::{prelude::*, winit::window::WindowId};

#[derive(PartialEq)]
pub(super) struct PlayerView {
    pub compact: bool,
    pub library_window: Option<WindowId>,
}
impl Component for PlayerView {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let playback = use_playback();
        let state = playback.snapshot();
        let compact = self.compact;
        let library_window = self.library_window;
        let mut expanded = playback.expanded;
        let mut panel = playback.panel;
        let panel_open = panel.read().is_some();
        let restore_size = use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(None)));
        use_side_effect_with_deps(&panel_open, move |open| {
            if !compact {
                return;
            }
            let open = *open;
            let restore_size = restore_size.clone();
            Platform::get().with_window(None, move |window| {
                if open {
                    let size = window.inner_size().to_logical::<f64>(window.scale_factor());
                    *restore_size.borrow_mut() = Some(size);
                    window
                        .set_min_inner_size(Some(freya::winit::dpi::LogicalSize::new(420., 550.)));
                    let _ = window.request_inner_size(freya::winit::dpi::LogicalSize::new(
                        size.width,
                        size.height + 320.,
                    ));
                } else {
                    window
                        .set_min_inner_size(Some(freya::winit::dpi::LogicalSize::new(420., 250.)));
                    if let Some(size) = restore_size.borrow_mut().take() {
                        let _ = window.request_inner_size(size);
                    }
                }
            });
        });
        let initial_size = *Platform::get().root_size.peek();
        let mut bounds = use_state(|| (initial_size.width, initial_size.height));
        let (view_width, view_height) = *bounds.read();
        let tight = !compact && view_height < 700.;
        let message = playback.error.read().clone().or(state.error.clone());
        let width = if panel_open && !compact {
            (view_width * 0.46).min(576.)
        } else {
            view_width.min(576.)
        };
        let art_size = if compact {
            72.
        } else {
            (view_height * 0.40)
                .min(380.)
                .min((width - 64.).max(64.))
                .min(
                    (view_height
                        - if tight { 408. } else { 468. }
                        - if message.is_some() { 28. } else { 0. })
                    .max(64.),
                )
        };
        let on_close = EventHandler::new(move |()| {
            if !compact {
                expanded.set(false);
                return;
            }
            let platform = Platform::get();
            let close = platform.clone();
            let _ = platform.post_callback(move |id, context| {
                if let Some(window) = library_window.and_then(|id| context.windows.get(&id)) {
                    window.window().set_minimized(false);
                    window.window().focus_window();
                }
                crate::ui::windows::manager::window_manager().close_player_window(true);
                close.close_window(id);
            });
        });
        let content = rect()
            .width(if compact {
                Size::fill()
            } else {
                Size::px(width)
            })
            .height(if compact && panel_open {
                Size::px(182.)
            } else {
                Size::fill()
            })
            .padding(if compact {
                0.
            } else if tight {
                16.
            } else {
                32.
            })
            .main_align(Alignment::Center)
            .spacing(if compact {
                8.
            } else if tight {
                12.
            } else {
                24.
            })
            .child(PlayerDetails {
                id: state.id,
                path: state.path.clone(),
                compact,
                tight,
                art_size,
            })
            .child(PlayerControls { compact })
            .maybe_child(message.map(|error| {
                label()
                    .text(error)
                    .font_size(11.)
                    .max_lines(2)
                    .color(colors.destructive)
            }));
        let close_on_escape = on_close.clone();
        VisibilityViewport::new().child(
            rect()
                .expanded()
                .overflow(Overflow::Clip)
                .on_sized(move |event: Event<SizedEventData>| {
                    bounds.set_if_modified((event.area.width(), event.area.height()));
                })
                .theme_background()
                .theme_color()
                .on_global_key_down(move |event: Event<KeyboardEventData>| {
                    if event.key == Key::Named(NamedKey::Escape) {
                        event.stop_propagation();
                        if panel.peek().is_some() {
                            panel.set(None);
                        } else {
                            close_on_escape.call(());
                        }
                    }
                })
                .child(
                    rect()
                        .position(Position::new_absolute().top(-64.).left(-64.))
                        .width(Size::px(view_width + 128.))
                        .height(Size::px(view_height + 128.))
                        .interactive(Interactive::No)
                        .opacity(0.4)
                        .child(ArtworkBackdrop {
                            file_id: state.id,
                            path: state.path.clone().unwrap_or_default(),
                            width: view_width + 128.,
                            height: view_height + 128.,
                        }),
                )
                .child(
                    rect()
                        .position(Position::new_absolute().top(0.).left(0.))
                        .width(Size::px(view_width))
                        .height(Size::px(view_height))
                        .interactive(Interactive::No)
                        .background(colors.surface_glass),
                )
                .child(
                    rect()
                        .expanded()
                        .content(Content::Flex)
                        .padding(if compact { 12. } else { 0. })
                        .child(PlayerHeader {
                            compact,
                            library_window,
                            on_close: on_close.clone(),
                        })
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::flex(1.))
                                .maybe(!compact, |row| row.horizontal())
                                .content(Content::Flex)
                                .main_align(Alignment::Center)
                                .child(content)
                                .maybe_child(
                                    panel_open
                                        .then_some(ExpandedPlaybackPanel { stacked: compact }),
                                ),
                        ),
                ),
        )
    }
}
