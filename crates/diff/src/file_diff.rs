use crate::parse_patch::parse_patch;
use crate::refine_changes::refine_changes;
use crate::{DiffHunk, DiffLineKind, DiffOptions};

/// One file's changes, ready to show: its hunks with numbered lines and
/// marked changes. Built from GitHub's per-file patch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileDiff {
    hunks: Vec<DiffHunk>,
}

impl FileDiff {
    /// Parses `patch` and applies `options`. A patch with only whitespace
    /// changes and `WhitespaceMode::Ignore` yields no hunks.
    pub fn build(patch: &str, options: DiffOptions) -> Self {
        Self {
            hunks: refine_changes(parse_patch(patch), options),
        }
    }

    pub fn hunks(&self) -> &[DiffHunk] {
        &self.hunks
    }

    pub fn is_empty(&self) -> bool {
        self.hunks.is_empty()
    }

    /// Added plus removed lines shown.
    pub fn changed_line_count(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .filter(|line| line.kind != DiffLineKind::Context)
            .count()
    }

    /// The largest line number on either side, which sizes the gutters.
    pub fn max_line_number(&self) -> u32 {
        self.hunks
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .flat_map(|line| [line.old_number, line.new_number])
            .flatten()
            .max()
            .unwrap_or_default()
    }
}
