use crate::{DiffLine, DiffLineKind, SplitRow};

/// A run of changed lines with their context, from one `@@` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// The `@@ -a,b +c,d @@ section` header line as written.
    pub header: String,
    pub lines: Vec<DiffLine>,
}

impl DiffHunk {
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
