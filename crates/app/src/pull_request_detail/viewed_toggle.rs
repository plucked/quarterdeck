use gpui::App;
use qd_cache::{ReviewScope, ViewedFile};

use crate::ui_state::UiStateRecorder;

/// What toggling a file's viewed checkbox did, for the caller to save.
#[derive(Debug, PartialEq, Eq)]
pub enum ViewedToggle {
    /// The file is now viewed at this fingerprint.
    Marked(ViewedFile),
    /// The file named here is no longer viewed.
    Unmarked(String),
}

impl ViewedToggle {
    /// Saves what the toggle did to the marks of `scope`.
    pub fn record(self, scope: ReviewScope, cx: &mut App) {
        match self {
            Self::Marked(mark) => UiStateRecorder::record_file_viewed(cx, scope, mark),
            Self::Unmarked(filename) => UiStateRecorder::record_file_unviewed(cx, scope, filename),
        }
    }
}
