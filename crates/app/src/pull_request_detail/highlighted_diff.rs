use std::sync::Arc;

use qd_diff::FileDiff;
use qd_syntax::DiffSyntax;

use super::loaded_file::LoadedFile;

/// The syntax colors computed for one diff, matched back by the diff's `Arc`.
pub struct HighlightedDiff {
    pub diff: Arc<FileDiff>,
    pub syntax: Option<Arc<DiffSyntax>>,
}

impl HighlightedDiff {
    /// Colors `loaded`'s diff by its language. Slow: background executor.
    pub fn of(loaded: LoadedFile) -> Self {
        Self {
            syntax: loaded.highlight_syntax(),
            diff: loaded.diff,
        }
    }
}
