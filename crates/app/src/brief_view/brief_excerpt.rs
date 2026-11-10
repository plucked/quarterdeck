use qd_diff::DiffLineKind;
use qd_studio::CodeAnchor;

use super::brief_diffs::BriefDiffs;
use crate::diff_line_sides::{line_number_on, line_side_of};

/// One line of code quoted in the brief, cut from the loaded diff.
pub struct ExcerptLine {
    pub number: u32,
    pub kind: DiffLineKind,
    pub text: String,
}

/// The lines `anchor` points at, in order, from the diff of its file: on
/// the base side the unchanged and removed lines, on the head side the
/// unchanged and added ones. Empty when the file is not loaded.
pub fn brief_excerpt(diffs: &dyn BriefDiffs, anchor: &CodeAnchor) -> Vec<ExcerptLine> {
    let Some(diff) = diffs.diff_of(&anchor.path) else {
        return Vec::new();
    };
    let lines = anchor.lines();
    let side = line_side_of(anchor.side);
    diff.hunks()
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter_map(|line| {
            let number = line_number_on(line, side)?;
            lines.contains(&number).then(|| ExcerptLine {
                number,
                kind: line.kind,
                text: line.text.clone(),
            })
        })
        .collect()
}
