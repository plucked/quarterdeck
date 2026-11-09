use gpui::{
    App, Bounds, Global, TitlebarOptions, WindowBounds, WindowHandle, WindowOptions, prelude::*,
    px, size,
};

use super::brief_gallery::BriefGallery;

const WINDOW_TITLE: &str = "Brief Gallery";
const WINDOW_WIDTH: f32 = 960.;
const WINDOW_HEIGHT: f32 = 860.;
const WINDOW_MIN_WIDTH: f32 = 420.;
const WINDOW_MIN_HEIGHT: f32 = 320.;

/// The gallery window last opened; it may have been closed since.
struct OpenBriefGallery(WindowHandle<BriefGallery>);

impl Global for OpenBriefGallery {}

/// Opens the brief gallery, or brings the open one to the front.
pub fn open_brief_gallery(cx: &mut App) {
    // A closed window no longer updates, so a failed update means opening a new one.
    if let Some(OpenBriefGallery(handle)) = cx.try_global::<OpenBriefGallery>()
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
            ..Default::default()
        },
        |window, cx| cx.new(|cx| BriefGallery::new(window, cx)),
    );
    match opened {
        Ok(handle) => cx.set_global(OpenBriefGallery(handle)),
        Err(error) => eprintln!("cannot open the brief gallery: {error:#}"),
    }
}
