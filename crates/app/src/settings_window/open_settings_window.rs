use gpui::{
    App, Bounds, Global, TitlebarOptions, WindowBounds, WindowHandle, WindowOptions, prelude::*,
    px, size,
};

use super::settings_window::SettingsWindow;

const WINDOW_TITLE: &str = "Settings";
const WINDOW_WIDTH: f32 = 720.;
const WINDOW_HEIGHT: f32 = 480.;
const WINDOW_MIN_WIDTH: f32 = 560.;
const WINDOW_MIN_HEIGHT: f32 = 360.;

/// The settings window last opened; it may have been closed since.
struct OpenSettingsWindow(WindowHandle<SettingsWindow>);

impl Global for OpenSettingsWindow {}

/// Opens the settings window, or brings it to the front when it is open.
pub fn open_settings_window(cx: &mut App) {
    // A closed window no longer updates, so a failed update means opening a new one.
    if let Some(OpenSettingsWindow(handle)) = cx.try_global::<OpenSettingsWindow>()
        && let handle = *handle
        && handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
    {
        return;
    }

    let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT))),
            titlebar: Some(TitlebarOptions {
                title: Some(WINDOW_TITLE.into()),
                ..Default::default()
            }),
            is_minimizable: false,
            ..Default::default()
        },
        |window, cx| cx.new(|cx| SettingsWindow::new(window, cx)),
    );
    match opened {
        Ok(handle) => cx.set_global(OpenSettingsWindow(handle)),
        Err(error) => eprintln!("cannot open the settings window: {error:#}"),
    }
}
