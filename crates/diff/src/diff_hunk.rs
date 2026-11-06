use std::ops::Range;

use crate::{DiffLine, DiffLineKind, SplitRow};

/// A run of changed lines with their context, from one `@@` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// The `@@ -a,b +c,d @@ section` header line as written.
    pub header: String,
    /// The header's `a` and `c`: where each side starts, or, for a side
    /// with no lines, the line after which the hunk sits.
    pub old_start: u32,
    pub new_start: u32,
    pub lines: Vec<DiffLine>,
}

impl DiffHunk {
    /// The new side's line numbers this hunk covers (end exclusive). A
    /// hunk with no new lines covers none, at the place after `new_start`.
    pub fn new_lines(&self) -> Range<u32> {
        side_lines(
            self.lines.iter().filter_map(|line| line.new_number),
            self.new_start,
        )
    }

    /// The old side's line numbers this hunk covers (end exclusive).
    pub fn old_lines(&self) -> Range<u32> {
        side_lines(
            self.lines.iter().filter_map(|line| line.old_number),
            self.old_start,
        )
    }

    /// The text after the header's closing `@@` (often the enclosing
    /// function), with its leading space; empty when there is none.
    pub fn section(&self) -> &str {
        self.header
            .strip_prefix("@@")
            .and_then(|rest| rest.split_once("@@"))
            .map_or("", |(_, section)| section)
    }

    /// The rows of a side-by-side view: context lines on both sides, and in
    /// each block of changes the n-th removed line beside the n-th added line.
    pub fn split_rows(&self) -> Vec<SplitRow> {
        let mut rows = Vec::with_capacity(self.lines.len());
        let mut index = 0;
        while index < self.lines.len() {
            if self.lines[index].kind == DiffLineKind::Context {
                rows.push(SplitRow {
                    old: Some(index),
                    new: Some(index),
                });
                index += 1;
                continue;
            }

            // A block: removed lines, then added lines, paired by position.
            let removed_start = index;
            while index < self.lines.len() && self.lines[index].kind == DiffLineKind::Removed {
                index += 1;
            }
            let added_start = index;
            while index < self.lines.len() && self.lines[index].kind == DiffLineKind::Added {
                index += 1;
            }
            let removed_count = added_start - removed_start;
            let added_count = index - added_start;
            for offset in 0..removed_count.max(added_count) {
                rows.push(SplitRow {
                    old: (offset < removed_count).then_some(removed_start + offset),
                    new: (offset < added_count).then_some(added_start + offset),
                });
            }
        }
        rows
    }

    /// Whether the hunk still changes anything (after folding whitespace changes).
    pub fn has_changes(&self) -> bool {
        self.lines
            .iter()
            .any(|line| line.kind != DiffLineKind::Context)
    }
}

/// The range from the first of a side's line `numbers` to one past the
/// last; for a side without lines, the empty range after `start` (a
/// header's start of an empty side names the line before it).
fn side_lines(mut numbers: impl Iterator<Item = u32>, start: u32) -> Range<u32> {
    let Some(first) = numbers.next() else {
        let place = start.saturating_add(1);
        return place..place;
    };
    let last = numbers.last().unwrap_or(first);
    first..last.saturating_add(1)
}
