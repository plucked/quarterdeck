use std::ops::Range;
use std::rc::Rc;

use super::{CodeSegments, DiffMatchReveal, DiffSide};

/// The selected part of one row's code: its column and byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideSelection {
    pub side: DiffSide,
    pub range: Range<usize>,
}

/// Find highlights for one column of a diff row, in ascending byte order.
#[derive(Clone, Default)]
pub struct CellMatches {
    pub ranges: Vec<Range<usize>>,
    pub current: Option<Range<usize>>,
    /// Shared with the Find bar so scrolling is requested only once for a navigation action.
    pub reveal: Option<Rc<DiffMatchReveal>>,
}

/// What a row's code cells need for selection: the row's index (they
/// register under it), where to register, and the row's selected part.
#[derive(Clone)]
pub struct CodeCells {
    pub row: usize,
    pub segments: CodeSegments,
    pub selection: Option<SideSelection>,
    /// Unified or old column, then the new column when split.
    pub matches: [CellMatches; 2],
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
