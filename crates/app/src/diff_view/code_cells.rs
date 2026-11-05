use std::ops::Range;

use super::{CodeSegments, DiffSide};

/// The selected part of one row's code: its column and byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideSelection {
    pub side: DiffSide,
    pub range: Range<usize>,
}

/// What a row's code cells need for selection: the row's index (they
/// register under it), where to register, and the row's selected part.
#[derive(Clone)]
pub struct CodeCells {
    pub row: usize,
    pub segments: CodeSegments,
    pub selection: Option<SideSelection>,
}

impl CodeCells {
    /// The selected byte range of column `side` in this row, if any.
    pub fn selected_range(&self, side: DiffSide) -> Option<Range<usize>> {
        self.selection
            .as_ref()
            .filter(|selection| selection.side == side)
            .map(|selection| selection.range.clone())
    }
}
