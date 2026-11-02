use crate::services::AppServices;
use audexis_audio::equalizer::{Bands, Mode, Settings, presets};
use freya::prelude::*;
use std::sync::Arc;

pub(crate) fn open(services: Arc<AppServices>, runtime: tokio::runtime::Handle) {
    let _ = Platform::get().post_callback(move |_, context| {
        let manager = super::manager::window_manager();
        if let Some(id) = manager.equalizer_window() {
            if let Some(window) = context.windows.get(&id) {
                window.window().set_minimized(false);
                window.window().focus_window();
                return;
            }
        }
        let on_close = manager.clone();
        let config = super::title_bar::configure(WindowConfig::new_app(EqualizerWindow {
            services,
            runtime,
        }))
        .with_title("Equalizer")
        .with_size(1120., 680.)
        .with_min_size(760., 620.)
        .with_on_close(move |_, _| {
            on_close.set_equalizer_window(None);
            CloseDecision::Close
        });
        manager.set_equalizer_window(Some(context.launch_window(config)));
    });
}

struct EqualizerWindow {
    services: Arc<AppServices>,
    runtime: tokio::runtime::Handle,
}
impl App for EqualizerWindow {
    fn render(&self) -> impl IntoElement {
        let services = self.services.clone();
        let runtime = self.runtime.clone();
        use_provide_context(move || services);
        use_provide_context(move || runtime);
        crate::ui::theme::use_app_theme(self.services.clone());
        EqualizerPanel
    }
}

#[derive(PartialEq)]
struct EqualizerPanel;
impl Component for EqualizerPanel {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let colors = crate::ui::theme::use_audexis_colors();
        let mut settings = use_state(|| *services.equalizer_updates.borrow());
        let mut error = use_state(|| services.equalizer_error.borrow().clone());
        let updates_services = services.clone();
        use_future(move || {
            let mut updates = updates_services.equalizer_updates.subscribe();
            let mut errors = updates_services.equalizer_error.subscribe();
            async move {
                loop {
                    tokio::select! {
                        result = updates.changed() => {
                            if result.is_err() { break; }
                            settings.set(*updates.borrow_and_update());
                        }
                        result = errors.changed() => {
                            if result.is_err() { break; }
                            error.set(errors.borrow_and_update().clone());
                        }
                    }
                }
            }
        });
        let audio_error = services.audio_error.clone();
        let on_change =
            EventHandler::new(move |next: Settings| match services.set_equalizer(next) {
                Ok(()) => {
                    settings.set(next);
                    error.set(None);
                }
                Err(message) => error.set(Some(message)),
            });
        let current = *settings.read();
        let enabled_change = on_change.clone();
        let reset_change = on_change.clone();
        let mut layouts = rect().horizontal().spacing(4.);
        for (bands, title) in [
            (Bands::Five, "5 bands"),
            (Bands::Ten, "10 bands"),
            (Bands::Twenty, "20 bands"),
        ] {
            let change = on_change.clone();
            layouts = layouts.child(
                Button::new()
                    .flat()
                    .compact()
                    .theme_colors(ButtonColorsThemePartial {
                        background: Some(
                            (if current.bands == bands {
                                colors.primary_washed
                            } else {
                                colors.background
                            })
                            .into(),
                        ),
                        color: Some(
                            (if current.bands == bands {
                                colors.primary
                            } else {
                                colors.muted_foreground
                            })
                            .into(),
                        ),
                        ..Default::default()
                    })
                    .child(title)
                    .on_press(move |_| {
                        let mut next = *settings.peek();
                        next.bands = bands;
                        change.call(next);
                    }),
            );
        }
        let custom_change = on_change.clone();
        let mut options = ScrollView::new()
            .width(Size::px(210.))
            .height(Size::px(300.))
            .child(
                MenuItem::new()
                    .selected(current.mode == Mode::Manual)
                    .child("Custom")
                    .on_press(move |_| {
                        let mut next = *settings.peek();
                        next.mode = Mode::Manual;
                        next.enabled = true;
                        custom_change.call(next);
                    }),
            );
        for info in presets() {
            let change = on_change.clone();
            options = options.child(
                MenuItem::new()
                    .selected(current.mode == Mode::Preset && current.preset == info.id)
                    .child(info.name)
                    .on_press(move |_| {
                        let mut next = *settings.peek();
                        next.preset = info.id;
                        next.mode = Mode::Preset;
                        next.enabled = true;
                        change.call(next);
                    }),
            );
        }
        let preset = Select::new()
            .selected_item(if current.mode == Mode::Manual {
                "Custom".to_owned()
            } else {
                current.preset.info().name.to_owned()
            })
            .child(options);
        let frequencies = current.bands.frequencies();
        let mut bank = rect()
            .horizontal()
            .width(Size::fill())
            .min_width(Size::px(frequencies.len() as f32 * 44. + 96.))
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(4.)
            .child(rect().width(Size::px(80.)).child(GainSlider {
                settings,
                index: None,
                on_change: on_change.clone(),
            }))
            .child(
                rect()
                    .width(Size::px(1.))
                    .height(Size::px(240.))
                    .background(colors.border),
            );
        for (index, _) in frequencies.iter().enumerate() {
            bank = bank.child(rect().width(Size::flex(1.)).child(GainSlider {
                settings,
                index: Some(index),
                on_change: on_change.clone(),
            }));
        }
        rect().expanded().content(Content::Flex).background(colors.background).color(colors.foreground)
            .maybe_child(cfg!(target_os = "macos").then_some(super::title_bar::TitleBar {
                child: label().text("Equalizer").into_element(),
            }))
            .child(ScrollView::new().width(Size::fill()).height(Size::flex(1.)).child(
                rect().width(Size::fill()).padding(24.).spacing(20.)
                    .child(rect().horizontal().width(Size::fill()).content(Content::Flex).cross_align(Alignment::Center)
                        .child(rect().width(Size::flex(1.)).spacing(6.)
                            .child(label().text("Shape your sound").font_size(22.))
                            .child(label().text(if current.enabled {
                                "EQ is on. Changes apply live and save automatically."
                            } else {
                                "EQ is off. Move a slider or choose a preset to turn it on."
                            }).font_size(12.).color(colors.muted_foreground)))
                        .child(Button::new().flat().child(rect().horizontal().spacing(8.).cross_align(Alignment::Center)
                            .child(Checkbox::new().selected(current.enabled))
                            .child(label().text(if current.enabled { "EQ on" } else { "EQ off" })))
                            .on_press(move |_| {
                                let mut next = *settings.peek();
                                next.enabled = !next.enabled;
                                enabled_change.call(next);
                            })))
                    .child(rect().horizontal().width(Size::fill()).content(Content::Flex).cross_align(Alignment::Center).spacing(12.)
                        .child(layouts).child(rect().width(Size::flex(1.)))
                        .child(preset)
                        .child(Button::new().outline().child("Reset").on_press(move |_| {
                            let mut next = *settings.peek(); next.reset_curve(); reset_change.call(next);
                        })))
                    .child(rect().width(Size::fill()).padding(16.).corner_radius(16.)
                        .background(colors.surface_subtle)
                        .child(ScrollView::new().direction(Direction::Horizontal).width(Size::fill()).height(Size::px(285.)).child(bank)))
                    .child(label().text("Bands: −12 to +12 dB. Lower the preamp when boosting to leave headroom.")
                        .font_size(12.).color(colors.muted_foreground))
                    .maybe_child(error.read().clone().or(audio_error).map(|message| label().text(message).font_size(12.).color(colors.destructive)))
            ))
    }
}

#[derive(PartialEq)]
struct GainSlider {
    settings: State<Settings>,
    index: Option<usize>,
    on_change: EventHandler<Settings>,
}
impl Component for GainSlider {
    fn render(&self) -> impl IntoElement {
        let settings = self.settings;
        let current = *settings.read();
        let colors = crate::ui::theme::use_audexis_colors();
        let index = self.index;
        let (gain, title) = if let Some(index) = index {
            let frequency = current.bands.frequencies()[index];
            (
                current.gains()[index],
                if frequency >= 1000. {
                    format!("{}k", frequency / 1000.)
                } else {
                    format!("{frequency}")
                },
            )
        } else {
            (current.preamp(), "Preamp".into())
        };
        let on_change = self.on_change.clone();
        rect()
            .width(Size::fill())
            .cross_align(Alignment::Center)
            .spacing(12.)
            .child(
                label()
                    .text(format!("{gain:+.1}"))
                    .font_size(11.)
                    .color(colors.muted_foreground),
            )
            .child(
                Slider::new(move |percent: f64| {
                    let mut next = *settings.peek();
                    let gain = ((percent * 0.24 - if index.is_some() { 12. } else { 24. }) * 10.)
                        .round()
                        / 10.;
                    if let Some(index) = index {
                        next.set_gain(index, gain);
                    } else {
                        next.set_preamp(gain);
                    }
                    next.enabled = true;
                    on_change.call(next);
                })
                .direction(Direction::Vertical)
                .size(Size::px(210.))
                .value((gain + if index.is_some() { 12. } else { 24. }) / 24. * 100.),
            )
            .child(label().text(title).font_size(11.))
    }
    fn render_key(&self) -> DiffKey {
        (&self.index).into()
    }
}
