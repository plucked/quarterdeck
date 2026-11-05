use std::ops::Range;

use crate::diff_view::{DiffPosition, DiffSide};

/// A text selection in the diff: from where the drag started (`anchor`) to
/// where the pointer is (`head`), in one code column. Positions are detail
/// rows and bytes, so the selection survives scrolling; it is cleared when
/// the rows change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffSelection {
    pub side: DiffSide,
    pub anchor: DiffPosition,
    pub head: DiffPosition,
}

impl DiffSelection {
    /// The selection's start and end in reading order.
    pub fn ordered(&self) -> (DiffPosition, DiffPosition) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// The selected bytes of row `row`, or `None` when the row is outside
    /// the selection. Rows after the first start at 0; rows before the last
    /// run to `usize::MAX`, which callers clamp to the line's length.
    pub fn range_in_row(&self, row: usize) -> Option<Range<usize>> {
        let (start, end) = self.ordered();
        if row < start.row || row > end.row {
            return None;
        }
        let from = if row == start.row { start.byte } else { 0 };
        let to = if row == end.row { end.byte } else { usize::MAX };
        Some(from..to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(row: usize, byte: usize) -> DiffPosition {
        DiffPosition { row, byte }
    }

    /// A backwards drag still covers its rows from start to end.
    #[test]
    fn backwards_drag_covers_rows_in_order() {
        let selection = DiffSelection {
            side: DiffSide::Unified,
            anchor: position(7, 2),
            head: position(5, 4),
        };
        assert_eq!(selection.range_in_row(5), Some(4..usize::MAX));
        assert_eq!(selection.range_in_row(6), Some(0..usize::MAX));
        assert_eq!(selection.range_in_row(7), Some(0..2));
        assert_eq!(selection.range_in_row(8), None);
    }

    /// Within one row the selection is the bytes between its ends.
    #[test]
    fn single_row_selection() {
        let selection = DiffSelection {
            side: DiffSide::Old,
            anchor: position(3, 9),
            head: position(3, 1),
        };
        assert_eq!(selection.range_in_row(3), Some(1..9));
        assert!(!selection.is_empty());
        let caret = DiffSelection {
            side: DiffSide::Old,
            anchor: position(3, 1),
            head: position(3, 1),
        };
        assert!(caret.is_empty());
    }
}
