use gpui::{App, Bounds, DisplayId, Pixels, Size, WindowBounds, px, size};

use crate::ui_state::{UiStateRecorder, window_bounds_from_placement};

/// The main window's size when no saved placement fits.
const WINDOW_WIDTH: f32 = 1200.;
const WINDOW_HEIGHT: f32 = 800.;

/// Where the main window opens.
pub struct RestoredWindow {
    pub bounds: WindowBounds,
    /// The display the bounds are relative to; `None` for the main display.
    pub display_id: Option<DisplayId>,
}

/// The saved placement on the display it was saved on, when that display
/// is connected and the window still fits it. A missing display falls back
/// to the main one with the same placement if it fits there; a placement
/// that fits nowhere opens centered at the default size.
pub fn restore_window(
    recorder: &UiStateRecorder,
    min_size: Size<Pixels>,
    cx: &App,
) -> RestoredWindow {
    let placement = recorder.saved_window_placement();

    // The saved display by its stable UUID, or else the main display.
    let saved_display = placement.as_ref().and_then(|placement| {
        let uuid = placement.display.as_ref()?;
        cx.displays().into_iter().find(|display| {
            display
                .uuid()
                .is_ok_and(|candidate| candidate.to_string() == uuid.as_str())
        })
    });
    let display = saved_display.or_else(|| cx.primary_display());
    let display_id = display.as_ref().map(|display| display.id());

    // The saved bounds if they fit that display, else centered on it.
    let bounds = placement
        .zip(display)
        .and_then(|(placement, display)| {
            window_bounds_from_placement(&placement, min_size, display.bounds().size)
        })
        .unwrap_or_else(|| {
            WindowBounds::Windowed(Bounds::centered(
                display_id,
                size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)),
                cx,
            ))
        });
    RestoredWindow { bounds, display_id }
}
