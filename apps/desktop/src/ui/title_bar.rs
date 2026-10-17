use freya::prelude::*;

const TITLE_BAR_HEIGHT: f32 = 60.0;

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
    pub title: &'static str,
}

impl Component for TitleBar {
    fn render(&self) -> impl IntoElement {
        rect()
            .width(Size::fill())
            .height(Size::px(TITLE_BAR_HEIGHT))
            .horizontal()
            .cross_align(Alignment::Center)
            .padding((0.0, 20.0, 0.0, 100.0))
            .window_drag()
            .child(label().text(self.title))
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
