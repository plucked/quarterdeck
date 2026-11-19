use qd_diff::{ChangeHighlight, DiffOptions, WhitespaceMode};

use crate::{DiffLayout, LineWrapping};

/// How file diffs are shown, saved across launches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiffViewSettings {
    pub layout: DiffLayout,
    pub highlight: ChangeHighlight,
    pub whitespace: WhitespaceMode,
    pub wrapping: LineWrapping,
    /// Open a comment on a double-click in a patch line. Off by default.
    pub double_click_to_comment: bool,
}

impl DiffViewSettings {
    /// The part of the settings that changes how a patch is built, not just drawn.
    pub fn diff_options(self) -> DiffOptions {
        DiffOptions {
            whitespace: self.whitespace,
            highlight: self.highlight,
        }
    }
}
