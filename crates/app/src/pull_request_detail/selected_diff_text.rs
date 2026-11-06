use super::detail_row::DetailRow;
use super::detail_row_list::DetailRowList;
use super::diff_selection::DiffSelection;
use super::loaded_file::LoadedFile;
use crate::diff_view::DiffSide;

/// The code a selection covers, as copied: each selected line's part in the
/// selection's column, joined by line breaks. Line numbers, markers, hunk
/// and file headers are not part of the code and are skipped, as are rows
/// with no line in that column.
pub fn selected_diff_text(
    selection: &DiffSelection,
    rows: &DetailRowList,
    files: &[LoadedFile],
) -> String {
    let (start, end) = selection.ordered();
    (start.row..=end.row)
        .filter_map(|row| {
            let text = row_code_text(rows.get(row)?, selection.side, files)?;
            let range = selection.range_in_row(row)?;
            let from = range.start.min(text.len());
            let to = range.end.min(text.len());
            Some(text.get(from..to).unwrap_or_default())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The code of `row` in column `side`; `None` for rows without code there.
pub fn row_code_text(row: DetailRow, side: DiffSide, files: &[LoadedFile]) -> Option<&str> {
    let (file, hunk, line) = match (row, side) {
        (DetailRow::UnifiedLine { file, hunk, line }, DiffSide::Unified) => (file, hunk, line),
        (DetailRow::SplitLine { file, hunk, row }, DiffSide::Old) => (file, hunk, row.old?),
        (DetailRow::SplitLine { file, hunk, row }, DiffSide::New) => (file, hunk, row.new?),
        _ => return None,
    };
    let diff_line = files.get(file)?.diff.hunks().get(hunk)?.lines.get(line)?;
    Some(diff_line.text.as_str())
}

#[cfg(test)]
mod tests {
    use qd_cache::{DiffLayout, DiffViewSettings};
    use qd_diff::DiffOptions;
    use qd_github::PullRequestFile;

    use super::*;
    use crate::diff_view::DiffPosition;
    use crate::pull_request_detail::changed_files::ChangedFiles;
    use crate::pull_request_detail::comment_placement::CommentPlacement;
    use crate::pull_request_detail::detail_content::DetailContent;
    use crate::pull_request_detail::file_review_state::FileReviewState;

    const PATCH: &str = "@@ -1,3 +1,3 @@\n keep one\n-old line\n+new line\n keep two\n";

    fn rows_for(layout: DiffLayout) -> (DetailRowList, Vec<LoadedFile>) {
        let files = vec![LoadedFile::build(
            PullRequestFile {
                filename: "a.rs".into(),
                previous_filename: None,
                status: "modified".into(),
                additions: 1,
                deletions: 1,
                patch: Some(PATCH.into()),
                sha: None,
            },
            DiffOptions::default(),
        )];
        let review = FileReviewState::for_loaded(&files, &[]);
        let changed = ChangedFiles::Loaded(files.clone());
        let shown = vec![true; files.len()];
        let rows = DetailRowList::build(&DetailContent {
            has_pull_request: true,
            changed_files: Some(&changed),
            shown_files: &shown,
            file_review: &review,
            settings: DiffViewSettings {
                layout,
                ..DiffViewSettings::default()
            },
            comments: &CommentPlacement::default(),
        });
        (rows, files)
    }

    /// The row index of the first diff line (after the summary, file and hunk headers).
    const FIRST_LINE_ROW: usize = 3;

    /// A unified selection copies the code of each line, without markers.
    #[test]
    fn copies_unified_lines() {
        let (rows, files) = rows_for(DiffLayout::Unified);
        let selection = DiffSelection {
            side: DiffSide::Unified,
            anchor: DiffPosition {
                row: FIRST_LINE_ROW,
                byte: 5,
            },
            head: DiffPosition {
                row: FIRST_LINE_ROW + 2,
                byte: 3,
            },
        };
        assert_eq!(
            selected_diff_text(&selection, &rows, &files),
            "one\nold line\nnew"
        );
    }

    /// A split selection copies only its column: the new side here.
    #[test]
    fn copies_one_side_of_split_rows() {
        let (rows, files) = rows_for(DiffLayout::Split);
        let selection = DiffSelection {
            side: DiffSide::New,
            anchor: DiffPosition {
                row: FIRST_LINE_ROW,
                byte: 0,
            },
            head: DiffPosition {
                row: FIRST_LINE_ROW + 2,
                byte: usize::MAX,
            },
        };
        assert_eq!(
            selected_diff_text(&selection, &rows, &files),
            "keep one\nnew line\nkeep two"
        );
    }
}
