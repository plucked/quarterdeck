use gpui::{Bounds, Pixels, Size, WindowBounds, point, px, size};
use qc_cache::{WindowMode, WindowPlacement};

/// Captures the window's current mode, position and size.
///
/// On macOS gpui opens a window with the given size as its content area but
/// reports `window_bounds` as the outer frame, title bar included. A windowed
/// window therefore saves `content_size` (the viewport), so reopening it
/// does not grow it by the title bar's height each time. Maximized and
/// fullscreen windows keep the restore bounds gpui reports.
pub fn placement_from_window(
    window_bounds: WindowBounds,
    content_size: Size<Pixels>,
) -> WindowPlacement {
    let (mode, bounds) = match window_bounds {
        WindowBounds::Windowed(frame) => (
            WindowMode::Windowed,
            Bounds::new(frame.origin, content_size),
        ),
        WindowBounds::Maximized(bounds) => (WindowMode::Maximized, bounds),
        WindowBounds::Fullscreen(bounds) => (WindowMode::Fullscreen, bounds),
    };
    WindowPlacement {
        mode,
        x: bounds.origin.x.into(),
        y: bounds.origin.y.into(),
        width: bounds.size.width.into(),
        height: bounds.size.height.into(),
    }
}

/// The bounds to open the window with, or `None` when the saved placement no
/// longer fits: not finite, smaller than `min_size`, or on no connected
/// display (e.g. an unplugged monitor).
pub fn window_bounds_from_placement(
    placement: &WindowPlacement,
    min_size: Size<Pixels>,
    displays: &[Bounds<Pixels>],
) -> Option<WindowBounds> {
    // Reject values a damaged row could hold.
    let values = [placement.x, placement.y, placement.width, placement.height];
    if values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let bounds = Bounds::new(
        point(px(placement.x), px(placement.y)),
        size(px(placement.width), px(placement.height)),
    );
    if bounds.size.width < min_size.width || bounds.size.height < min_size.height {
        return None;
    }

    // The window must overlap a display that exists now.
    if !displays.iter().any(|display| display.intersects(&bounds)) {
        return None;
    }

    Some(match placement.mode {
        WindowMode::Windowed => WindowBounds::Windowed(bounds),
        WindowMode::Maximized => WindowBounds::Maximized(bounds),
        WindowMode::Fullscreen => WindowBounds::Fullscreen(bounds),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display() -> Bounds<Pixels> {
        Bounds::new(point(px(0.), px(0.)), size(px(1920.), px(1080.)))
    }

    fn min_size() -> Size<Pixels> {
        size(px(720.), px(480.))
    }

    fn windowed(x: f32, y: f32, width: f32, height: f32) -> WindowPlacement {
        WindowPlacement {
            mode: WindowMode::Windowed,
            x,
            y,
            width,
            height,
        }
    }

    /// A window on a connected display is restored as it was.
    #[test]
    fn restores_window_on_connected_display() {
        let placement = windowed(100., 100., 1200., 800.);
        let restored = window_bounds_from_placement(&placement, min_size(), &[display()]).unwrap();
        let content_size = restored.get_bounds().size;
        assert_eq!(placement_from_window(restored, content_size), placement);
    }

    /// A windowed window saves its content size, not the taller frame, so it
    /// does not grow by the title bar on every relaunch.
    #[test]
    fn windowed_placement_saves_content_size() {
        let frame = Bounds::new(point(px(100.), px(100.)), size(px(1200.), px(832.)));
        let content_size = size(px(1200.), px(800.));
        let placement = placement_from_window(WindowBounds::Windowed(frame), content_size);
        assert_eq!(placement, windowed(100., 100., 1200., 800.));
    }

    /// A window on a display that is gone falls back to the default placement.
    #[test]
    fn window_on_missing_display_falls_back() {
        let placement = windowed(5000., 100., 1200., 800.);
        assert!(window_bounds_from_placement(&placement, min_size(), &[display()]).is_none());
    }

    /// A window smaller than the minimum size falls back.
    #[test]
    fn window_below_minimum_size_falls_back() {
        let placement = windowed(0., 0., 100., 100.);
        assert!(window_bounds_from_placement(&placement, min_size(), &[display()]).is_none());
    }

    /// Non-finite values from a damaged row fall back instead of opening a broken window.
    #[test]
    fn non_finite_values_fall_back() {
        let placement = windowed(f32::NAN, 0., 1200., 800.);
        assert!(window_bounds_from_placement(&placement, min_size(), &[display()]).is_none());
    }
}
