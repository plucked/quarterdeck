//! A diff line's two sides: `line_number_on` reads its number on one, and
//! `line_side_of` names a brief anchor's side as a review comment's.

use qd_diff::DiffLine;
use qd_github::LineSide;
use qd_studio::CodeSide;

/// `line`'s number in the old file (`LineSide::Old`) or the new one
/// (`LineSide::New`); `None` when the line is not on that side (an added
/// line has no old number, a removed one no new number).
pub fn line_number_on(line: &DiffLine, side: LineSide) -> Option<u32> {
    match side {
        LineSide::Old => line.old_number,
        LineSide::New => line.new_number,
    }
}

/// The side of the diff a brief's anchor points at: its base is the old
/// side, its head the new one.
pub fn line_side_of(side: CodeSide) -> LineSide {
    match side {
        CodeSide::Base => LineSide::Old,
        CodeSide::Head => LineSide::New,
    }
}
