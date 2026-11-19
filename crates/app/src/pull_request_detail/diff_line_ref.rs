use qd_diff::{DiffLine, LineOrigin};
use qd_github::LineSide;

use crate::diff_line_sides::line_number_on;
use crate::diff_view::DiffSide;

/// One line of the diff a comment can sit under: line `line` on `side` of
/// the loaded file `file`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiffLineRef {
    pub file: usize,
    pub side: LineSide,
    pub line: u32,
}

impl DiffLineRef {
    /// Prefer the new side for shared context or unified rows with two numbers.
    pub fn on_patch(file: usize, column: DiffSide, line: &DiffLine) -> Option<Self> {
        [LineSide::New, LineSide::Old].into_iter().find_map(|side| {
            let line = Self::patch_number(line, column, side)?;
            Some(Self { file, side, line })
        })
    }

    /// Which original patch coordinates the displayed cell can contribute to a comment.
    pub fn patch_number(line: &DiffLine, column: DiffSide, side: LineSide) -> Option<u32> {
        match (line.origin, column, side) {
            (LineOrigin::FileText, _, _)
            | (LineOrigin::FoldedReplacement, DiffSide::Old, LineSide::New)
            | (LineOrigin::FoldedReplacement, DiffSide::New, LineSide::Old) => None,
            _ => line_number_on(line, side),
        }
    }
}
