use freya::prelude::*;

const TITLE_BAR_HEIGHT: f32 = 60.0;
const PREVIEW_WINDOWS_TITLE_BAR_ON_MAC: bool = false;

pub fn windows_title_bar() -> bool {
    cfg!(target_os = "windows") || (cfg!(target_os = "macos") && PREVIEW_WINDOWS_TITLE_BAR_ON_MAC)
}

pub fn configure(config: WindowConfig) -> WindowConfig {
    #[cfg(target_os = "macos")]
    {
        use freya::winit::platform::macos::WindowAttributesExtMacOS;

        config
            .with_decorations(true)
            .with_window_attributes(|attributes, _| {
                attributes
                    .with_titlebar_transparent(true)
                    .with_title_hidden(true)
                    .with_fullsize_content_view(true)
                    .with_titlebar_buttons_hidden(false)
            })
            .with_window_handle(install_native_toolbar)
    }
    #[cfg(not(target_os = "macos"))]
    config
}

#[derive(PartialEq)]
pub struct TitleBar {
    pub child: Element,
}

impl Component for TitleBar {
    fn render(&self) -> impl IntoElement {
        let geometry = use_title_geometry();
        let colors = crate::ui::theme::use_audexis_colors();
        rect()
            .width(Size::fill())
            .height(Size::px(geometry.0))
            .horizontal()
            .cross_align(Alignment::Center)
            .padding((0.0, 20.0, 0.0, geometry.1))
            .border(
                Border::new()
                    .width(BorderWidth {
                        bottom: 1.0,
                        ..Default::default()
                    })
                    .fill(colors.border),
            )
            .window_drag()
            .child(self.child.clone())
    }
}

#[cfg(target_os = "macos")]
fn install_native_toolbar(window: &mut freya::winit::window::Window) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSToolbar, NSView, NSWindowToolbarStyle};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };

    let view = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
    let Some(native_window) = view.window() else {
        return;
    };

    let toolbar = NSToolbar::new(mtm);
    #[allow(deprecated)]
    toolbar.setShowsBaselineSeparator(false);
    native_window.setToolbar(Some(&toolbar));
    native_window.setToolbarStyle(NSWindowToolbarStyle::Unified);
}

fn use_title_geometry() -> (f32, f32) {
    let mut geometry = use_state(|| (TITLE_BAR_HEIGHT, 100.0));
    #[cfg(target_os = "macos")]
    use_hook(move || {
        let (tx, rx) = tokio::sync::oneshot::channel();
        Platform::get().with_window(None, move |window| {
            let _ = tx.send(native_title_geometry(window));
        });
        spawn(async move {
            if let Ok(Some(value)) = rx.await {
                geometry.set(value);
            }
        });
    });
    *geometry.read()
}

#[cfg(target_os = "macos")]
fn native_title_geometry(window: &freya::winit::window::Window) -> Option<(f32, f32)> {
    use objc2_app_kit::{NSView, NSWindowButton};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let RawWindowHandle::AppKit(handle) = window.window_handle().ok()?.as_raw() else {
        return None;
    };
    let view = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
    let native_window = view.window()?;
    let button = native_window.standardWindowButton(NSWindowButton::ZoomButton)?;
    let bounds = button.bounds();
    let mut point = bounds.origin;
    point.x += bounds.size.width;
    point.y += bounds.size.height / 2.0;
    let point = view.convertPoint_fromView(point, Some(&button));
    let bounds = view.bounds();
    let center = if view.isFlipped() {
        point.y - bounds.origin.y
    } else {
        bounds.origin.y + bounds.size.height - point.y
    };
    if !center.is_finite() || center <= 0.0 {
        return None;
    }
    Some((
        (center * 2.0) as f32,
        (point.x - bounds.origin.x + 16.0) as f32,
    ))
}
