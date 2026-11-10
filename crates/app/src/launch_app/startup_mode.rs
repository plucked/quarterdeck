use std::path::PathBuf;

use gpui::WindowBackgroundAppearance;

use crate::main_window::MainWindowReveal;
use crate::startup_splash::startup_video_path;

/// How the main window comes up.
pub enum StartupMode {
    /// Behind the startup splash playing this movie: opened hidden and
    /// see-through, revealed when the movie ends.
    Splash(PathBuf),
    /// At once (the movie is missing).
    Direct,
}

impl StartupMode {
    /// The splash when its movie is there.
    pub fn detect() -> Self {
        startup_video_path().map_or(Self::Direct, Self::Splash)
    }

    pub fn shows_main_window_at_once(&self) -> bool {
        matches!(self, Self::Direct)
    }

    /// See-through while hidden behind the splash, so it can fade in.
    pub fn main_window_background(&self) -> WindowBackgroundAppearance {
        match self {
            Self::Splash(_) => WindowBackgroundAppearance::Transparent,
            Self::Direct => WindowBackgroundAppearance::Opaque,
        }
    }

    pub fn main_window_reveal(&self) -> MainWindowReveal {
        match self {
            Self::Splash(_) => MainWindowReveal::AfterSplash,
            Self::Direct => MainWindowReveal::Shown,
        }
    }
}
