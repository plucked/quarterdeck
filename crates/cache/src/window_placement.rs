use crate::WindowMode;

/// The main window's mode and its windowed (restore) bounds in screen
/// points, as last seen. Whether it still fits the connected displays is for
/// the app to check when restoring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowPlacement {
    pub mode: WindowMode,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
