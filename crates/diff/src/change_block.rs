use std::ops::Range;

use crate::{DiffLine, DiffLineKind};

/// A run of removed lines followed by the run of added lines that replaces
/// them, as index ranges into a hunk's lines. Either run may be empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChangeBlock {
    pub removed: Range<usize>,
    pub added: Range<usize>,
}

impl ChangeBlock {
    /// Every block of `lines`, in order; context lines separate blocks.
    pub fn find_all(lines: &[DiffLine]) -> Vec<Self> {
        let mut blocks = Vec::new();
        let mut index = 0;
        while index < lines.len() {
            if lines[index].kind == DiffLineKind::Context {
                index += 1;
                continue;
            }
            let removed_start = index;
            while index < lines.len() && lines[index].kind == DiffLineKind::Removed {
                index += 1;
            }
            let added_start = index;
            while index < lines.len() && lines[index].kind == DiffLineKind::Added {
                index += 1;
            }
            blocks.push(Self {
                removed: removed_start..added_start,
                added: added_start..index,
            });
        }
        blocks
    }

    /// Pairs of (removed, added) line indexes, the n-th of each run together.
    pub fn pairs(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.removed.clone().zip(self.added.clone())
    }

    /// The block's first and one-past-last line index.
    pub fn span(&self) -> Range<usize> {
        self.removed.start..self.added.end
    }
}
