use qd_cache::{DiffLayout, DiffViewSettings, LineWrapping};
use qd_diff::DiffHunk;

use super::detail_row::DetailRow;
use super::detail_row_list::DetailRowList;
use super::loaded_file::LoadedFile;
use crate::diff_view::{DIFF_CHARACTER_WIDTH, DIFF_ROW_HEIGHT};
use crate::scrollbar::ListRowTops;

/// Heights of the rows that do not depend on their content, as gpui lays them out.
const SUMMARY_BAR_HEIGHT: f32 = 53.5;
const FILE_HEADER_HEIGHT: f32 = 37.5;
const FILE_NOTE_HEIGHT: f32 = 34.;
const FILE_END_HEIGHT: f32 = 16.;
/// Typical heights of rows that do depend on their content.
const COMMENT_CARD_HEIGHT: f32 = 110.;
const COMPOSER_HEIGHT: f32 = 170.;
const STATUS_ROW_HEIGHT: f32 = 156.;
/// Width around a line's code: the pane's padding, the card's borders, the
/// comment button and the `+`/`-` marker.
const CODE_INSET: f32 = 32. + 2. + 14. + 14.;

/// Where every row of the detail list starts, estimated without laying any
/// out, for the scrollbar: the list lays out only the rows on screen, so a
/// pull request of thousands of files opens without measuring each line.
/// Fixed-height rows are exact; a wrapped line counts the rows its text
/// fills at `pane_width`; comment cards take a typical height.
pub fn estimate_row_tops(
    rows: &DetailRowList,
    files: &[LoadedFile],
    settings: DiffViewSettings,
    pane_width: f32,
) -> ListRowTops {
    ListRowTops::from_heights(
        (0..rows.len())
            .filter_map(|index| rows.get(index))
            .map(|row| estimated_height(row, files, settings, pane_width)),
    )
}

/// One row's estimated height.
fn estimated_height(
    row: DetailRow,
    files: &[LoadedFile],
    settings: DiffViewSettings,
    pane_width: f32,
) -> f32 {
    match row {
        DetailRow::NoSelection
        | DetailRow::FilesLoading
        | DetailRow::FilesFailed
        | DetailRow::AllFilesHidden => STATUS_ROW_HEIGHT,
        DetailRow::SummaryBar => SUMMARY_BAR_HEIGHT,
        DetailRow::FileHeader { .. } => FILE_HEADER_HEIGHT,
        DetailRow::FileNote { .. } => FILE_NOTE_HEIGHT,
        DetailRow::HunkHeader { .. } => DIFF_ROW_HEIGHT,
        DetailRow::FileEnd { .. } => FILE_END_HEIGHT,
        DetailRow::CommentCard { .. } => COMMENT_CARD_HEIGHT,
        DetailRow::Composer { .. } => COMPOSER_HEIGHT,
        DetailRow::UnifiedLine { file, hunk, line } => {
            let Some((loaded, hunk)) = file_hunk(files, file, hunk) else {
                return DIFF_ROW_HEIGHT;
            };
            let code_width = pane_width - CODE_INSET - 2. * loaded.gutter_width;
            line_height(hunk, Some(line), code_width, settings)
        }
        DetailRow::SplitLine { file, hunk, row } => {
            let Some((loaded, hunk)) = file_hunk(files, file, hunk) else {
                return DIFF_ROW_HEIGHT;
            };
            let code_width = pane_width / 2. - CODE_INSET / 2. - loaded.gutter_width;
            line_height(hunk, row.old, code_width, settings)
                .max(line_height(hunk, row.new, code_width, settings))
        }
    }
}

/// The loaded file `file` and its hunk `hunk`.
fn file_hunk(files: &[LoadedFile], file: usize, hunk: usize) -> Option<(&LoadedFile, &DiffHunk)> {
    let loaded = files.get(file)?;
    let hunk = loaded.diff.hunks().get(hunk)?;
    Some((loaded, hunk))
}

/// The height of line `line` of `hunk` in a code column `code_width` wide:
/// one diff row unless wrapping spreads it over several.
fn line_height(
    hunk: &DiffHunk,
    line: Option<usize>,
    code_width: f32,
    settings: DiffViewSettings,
) -> f32 {
    let characters = line
        .and_then(|line| hunk.lines.get(line))
        .map_or(0, |line| line.text.chars().count());
    let rows = match (settings.wrapping, settings.layout) {
        (LineWrapping::Scroll, DiffLayout::Unified | DiffLayout::Split) => 1.,
        (LineWrapping::Wrap, DiffLayout::Unified | DiffLayout::Split) => {
            wrapped_rows(characters, code_width)
        }
    };
    DIFF_ROW_HEIGHT * rows
}

/// Rows `characters` of code fill in a column `code_width` wide; one when
/// the width is not known yet (before the first layout).
fn wrapped_rows(characters: usize, code_width: f32) -> f32 {
    let per_row = (code_width / DIFF_CHARACTER_WIDTH).floor();
    if per_row < 1. {
        return 1.;
    }
    let characters = f32::from(u16::try_from(characters).unwrap_or(u16::MAX));
    (characters / per_row).ceil().max(1.)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line wraps into as many rows as its characters fill.
    #[test]
    fn wraps_long_lines() {
        let per_row_width = DIFF_CHARACTER_WIDTH * 10.;
        assert!((wrapped_rows(0, per_row_width) - 1.).abs() < f32::EPSILON);
        assert!((wrapped_rows(10, per_row_width) - 1.).abs() < f32::EPSILON);
        assert!((wrapped_rows(11, per_row_width) - 2.).abs() < f32::EPSILON);
        assert!((wrapped_rows(500, 0.) - 1.).abs() < f32::EPSILON);
    }
}
