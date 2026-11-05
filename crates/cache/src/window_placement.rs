use crate::{DisplayUuid, WindowMode};

/// The main window's mode, the display it was on, and its windowed
/// (restore) bounds in screen points relative to that display's top-left
/// corner, as last seen. Whether it still fits is for the app to check when
/// restoring.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowPlacement {
    pub mode: WindowMode,
    /// `None` when the display could not be identified; the window then
    /// opens on the main display.
    pub display: Option<DisplayUuid>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
