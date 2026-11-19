use qd_github::LineSide;

use super::comment_range::CommentRange;
use super::detail_row::DetailRow;
use super::detail_row_list::DetailRowList;
use super::diff_line_ref::DiffLineRef;
use super::diff_selection::DiffSelection;
use super::loaded_file::LoadedFile;
use super::selected_diff_text::row_code_line;

/// Maps a multiline text selection to consecutive source lines on a single side of one patch hunk.
pub(super) fn selected_comment_range(
    selection: DiffSelection,
    rows: &DetailRowList,
    files: &[LoadedFile],
) -> Option<CommentRange> {
    let (start, end) = selection.ordered();
    // The last line contributes no text when the selection finishes at its first character.
    let last_row = end.row.checked_sub(usize::from(end.byte == 0))?;
    if start.row >= last_row {
        return None;
    }
    let file = rows.file_at_row(start.row)?;
    let selected = rows.rows_in(start.row..last_row.checked_add(1)?);
    // Context shown twice has one canonical GitHub location on the right. A selection
    // containing deletions uses the old line numbers; mixed additions and deletions have no side.
    [LineSide::New, LineSide::Old].into_iter().find_map(|side| {
        let mut range: Option<CommentRange> = None;
        for &row in selected {
            match row {
                DetailRow::CommentCard { .. } | DetailRow::Composer { .. } => continue,
                DetailRow::UnifiedLine { .. } | DetailRow::SplitLine { .. } => {}
                _ => return None,
            }
            let Some(line) = row_code_line(row, selection.side, files) else {
                // A filler in this split column has no source line.
                continue;
            };
            let number = DiffLineRef::patch_number(line, selection.side, side)?;
            match &mut range {
                None => range = Some(CommentRange::ending_at(file, side, number, None)),
                Some(range) if range.last.checked_add(1) == Some(number) => range.last = number,
                Some(_) => return None,
            }
        }
        range.filter(|range| range.first < range.last && range.in_patch(&files[file]))
    })
}

#[cfg(test)]
mod tests {
    use qd_cache::{DiffLayout, DiffViewSettings};
    use qd_github::PullRequestFile;

    use super::*;
    use crate::diff_view::{DiffPosition, DiffSide};
    use crate::pull_request_detail::comment_placement::CommentPlacement;
    use crate::pull_request_detail::file_review_state::FileReviewState;

    const PATCH: &str = "@@ -10,4 +20,5 @@\n context one\n-old one\n-old two\n+new one\n+new two\n+new extra\n context two\n@@ -30 +40 @@\n-next\n+next version";

    /// One expanded file with distinct coordinates and an unpaired added line in a split hunk.
    fn rows_for(layout: DiffLayout) -> (DetailRowList, Vec<LoadedFile>) {
        let files = vec![LoadedFile::build(
            PullRequestFile {
                filename: "file".into(),
                previous_filename: None,
                status: "modified".into(),
                additions: 4,
                deletions: 3,
                patch: Some(PATCH.into()),
                sha: None,
            },
            qd_diff::DiffOptions::default(),
        )];
        let rows = DetailRowList::group(
            &[0],
            &[0].into(),
            &files,
            &FileReviewState::default(),
            DiffViewSettings {
                layout,
                ..DiffViewSettings::default()
            },
            &CommentPlacement::default(),
        );
        (rows, files)
    }

    fn selection(side: DiffSide, start: usize, end: usize) -> DiffSelection {
        DiffSelection {
            side,
            anchor: DiffPosition {
                row: start,
                byte: 1,
            },
            head: DiffPosition { row: end, byte: 3 },
        }
    }

    /// Each split column uses its own numbers, handles backward drags and skips an empty end row.
    #[test]
    fn selects_a_range_on_the_intended_split_side() {
        let (rows, files) = rows_for(DiffLayout::Split);
        let selected = |selection| selected_comment_range(selection, &rows, &files);
        assert_eq!(
            selected(selection(DiffSide::Old, 4, 3)),
            Some(CommentRange::ending_at(0, LineSide::Old, 12, Some(11)))
        );
        assert_eq!(
            selected(selection(DiffSide::New, 2, 6)),
            Some(CommentRange::ending_at(0, LineSide::New, 24, Some(20)))
        );
        let mut before_last = selection(DiffSide::New, 3, 5);
        before_last.head.byte = 0;
        assert_eq!(
            selected(before_last),
            Some(CommentRange::ending_at(0, LineSide::New, 22, Some(21)))
        );
    }

    /// A drag across both unified change sides or two original patch hunks cannot form one anchor.
    #[test]
    fn rejects_mixed_unified_sides_and_hunk_boundaries() {
        let (rows, files) = rows_for(DiffLayout::Unified);
        let selected = |selection| selected_comment_range(selection, &rows, &files);
        assert!(selected(selection(DiffSide::Unified, 3, 5)).is_none());
        assert!(selected(selection(DiffSide::Unified, 5, 11)).is_none());
        assert_eq!(
            selected(selection(DiffSide::Unified, 5, 7)),
            Some(CommentRange::ending_at(0, LineSide::New, 23, Some(21)))
        );
    }
}
