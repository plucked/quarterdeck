use std::ops::Range;

use qd_diff::FileDiff;
use regex::RegexBuilder;

use super::detail_row::DetailRow;
use crate::diff_view::DiffSide;

/// One occurrence in the displayed file. Mirrored split context belongs to both columns.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct DiffFindMatch {
    pub row: usize,
    pub side: DiffSide,
    pub range: Range<usize>,
    pub mirrored: bool,
}

/// Literal, case-insensitive occurrences ordered by displayed row, column and byte.
#[derive(Default)]
pub(super) struct DiffFindMatches {
    pub occurrences: Vec<DiffFindMatch>,
}

impl DiffFindMatches {
    /// Scans the file's displayed code, including expanded context and offscreen rows.
    pub fn search(diff: &FileDiff, rows: &[DetailRow], query: &str) -> Self {
        let mut matches = Self::default();
        if query.is_empty() {
            return matches;
        }
        let Ok(pattern) = RegexBuilder::new(&regex::escape(query))
            .case_insensitive(true)
            .build()
        else {
            return matches;
        };
        for (row, detail) in rows.iter().enumerate() {
            // A split context line has one source location even though both columns show it.
            let mut scan = |hunk: usize, line: usize, side, mirrored| {
                let text = &diff.hunks()[hunk].lines[line].text;
                matches
                    .occurrences
                    .extend(pattern.find_iter(text).map(|found| DiffFindMatch {
                        row,
                        side,
                        mirrored,
                        range: found.range(),
                    }));
            };
            match *detail {
                DetailRow::UnifiedLine { hunk, line, .. } => {
                    scan(hunk, line, DiffSide::Unified, false);
                }
                DetailRow::SplitLine {
                    hunk, row: pair, ..
                } => {
                    if let Some(old) = pair.old {
                        scan(hunk, old, DiffSide::Old, pair.old == pair.new);
                    }
                    if let Some(new) = pair.new
                        && pair.old != pair.new
                    {
                        scan(hunk, new, DiffSide::New, false);
                    }
                }
                _ => {}
            }
        }
        matches
    }

    /// Occurrences on one row, with their positions in the navigation order.
    pub fn on_row(&self, row: usize) -> impl Iterator<Item = (usize, &DiffFindMatch)> {
        let first = self.occurrences.partition_point(|found| found.row < row);
        self.occurrences[first..]
            .iter()
            .take_while(move |found| found.row == row)
            .enumerate()
            .map(move |(offset, found)| (first + offset, found))
    }
}

#[cfg(test)]
mod tests {
    use qd_diff::{DiffOptions, SplitRow};

    use super::*;

    /// Metacharacters match literally and Unicode case folding returns original byte ranges.
    #[test]
    fn finds_repeated_literals_in_unified_code() {
        let diff = FileDiff::build("@@ -1 +1 @@\n-old\n+π.É.é\n", DiffOptions::default());
        let rows = [
            DetailRow::FileHeader { file: 1 },
            DetailRow::UnifiedLine {
                file: 1,
                hunk: 0,
                line: 1,
            },
        ];
        let found = DiffFindMatches::search(&diff, &rows, ".é");
        assert_eq!(
            found
                .on_row(1)
                .map(|(_, found)| found.range.clone())
                .collect::<Vec<_>>(),
            [2..5, 5..8]
        );
        assert!(found.on_row(0).next().is_none());
        assert_eq!(
            DiffFindMatches::search(&diff, &rows, "").occurrences.len(),
            0
        );
    }

    /// Split context counts once and each changed side is visited in row order.
    #[test]
    fn split_context_has_one_stop_and_both_changed_sides_match() {
        let diff = FileDiff::build(
            "@@ -1,2 +1,2 @@\n hit context\n-hit old\n+hit new\n",
            DiffOptions::default(),
        );
        let split = |old, new| DetailRow::SplitLine {
            file: 1,
            hunk: 0,
            row: SplitRow {
                old: Some(old),
                new: Some(new),
            },
        };
        let rows = [split(0, 0), split(1, 2)];
        let hits = DiffFindMatches::search(&diff, &rows, "HIT").occurrences;
        assert_eq!(hits.len(), 3);
        assert!(hits[0].mirrored);
        assert_eq!(hits[1].side, DiffSide::Old);
        assert_eq!(hits[2].side, DiffSide::New);
        assert!(!hits[1].mirrored && !hits[2].mirrored);
    }
}
