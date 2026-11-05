use std::ops::Range;

use qc_cache::{DiffLayout, DiffViewSettings};
use qc_diff::WhitespaceMode;

use super::changed_files::ChangedFiles;
use super::detail_content::DetailContent;
use super::detail_row::DetailRow;
use super::loaded_file::LoadedFile;
use crate::diff_view::{FileDiffState, FileNote};

/// The rows of the detail pane in order, and which rows belong to each file,
/// so one file can be collapsed or expanded without rebuilding the rest.
#[derive(Debug, Default)]
pub struct DetailRowList {
    rows: Vec<DetailRow>,
    /// Row range of each loaded file, by file index.
    file_ranges: Vec<Range<usize>>,
}

impl DetailRowList {
    /// The rows for `content`: the summary, then the rows of every file the
    /// filter shows (a hidden file keeps an empty range where its rows would
    /// go), or a loading, failure or all-hidden row instead.
    pub fn build(content: &DetailContent<'_>) -> Self {
        let mut list = Self::default();
        if !content.has_pull_request {
            list.rows.push(DetailRow::NoSelection);
            return list;
        }
        match content.changed_files {
            None | Some(ChangedFiles::Loading(_)) => list.rows.push(DetailRow::FilesLoading),
            Some(ChangedFiles::Failed(_)) => list.rows.push(DetailRow::FilesFailed),
            Some(ChangedFiles::Loaded(files)) => {
                list.rows.push(DetailRow::SummaryBar);
                for (index, loaded) in files.iter().enumerate() {
                    let start = list.rows.len();
                    if content.shown_files.get(index).copied().unwrap_or(true) {
                        let state = content.file_review.state_of(loaded);
                        list.rows
                            .extend(Self::file_rows(index, loaded, state, content.settings));
                    }
                    list.file_ranges.push(start..list.rows.len());
                }
                if !files.is_empty() && !content.shown_files.contains(&true) {
                    list.rows.push(DetailRow::AllFilesHidden);
                }
            }
        }
        list
    }

    /// The rows of one file: its header, then its lines (or a note), then
    /// the strip closing its card.
    pub fn file_rows(
        file: usize,
        loaded: &LoadedFile,
        state: FileDiffState,
        settings: DiffViewSettings,
    ) -> Vec<DetailRow> {
        let mut rows = vec![DetailRow::FileHeader { file }];
        if let Some(note) = file_note(loaded, state, settings) {
            rows.push(DetailRow::FileNote { file, note });
        } else if !state.is_collapsed {
            for (hunk_index, hunk) in loaded.diff.hunks().iter().enumerate() {
                rows.push(DetailRow::HunkHeader {
                    file,
                    hunk: hunk_index,
                });
                match settings.layout {
                    DiffLayout::Unified => {
                        rows.extend((0..hunk.lines.len()).map(|line| DetailRow::UnifiedLine {
                            file,
                            hunk: hunk_index,
                            line,
                        }));
                    }
                    DiffLayout::Split => {
                        rows.extend(hunk.split_rows().into_iter().map(|row| {
                            DetailRow::SplitLine {
                                file,
                                hunk: hunk_index,
                                row,
                            }
                        }));
                    }
                }
            }
        }
        rows.push(DetailRow::FileEnd { file });
        rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn get(&self, index: usize) -> Option<DetailRow> {
        self.rows.get(index).copied()
    }

    /// The first row (the header) of file `file`; `None` while it is hidden.
    pub fn file_start(&self, file: usize) -> Option<usize> {
        self.file_ranges
            .get(file)
            .filter(|range| !range.is_empty())
            .map(|range| range.start)
    }

    /// The file whose rows include row `row`; `None` above the files.
    pub fn file_at_row(&self, row: usize) -> Option<usize> {
        self.file_ranges
            .iter()
            .position(|range| range.contains(&row))
    }

    /// Replaces the rows of file `file` with `rows`, moving the later files'
    /// ranges. Returns the replaced range, for splicing the list the same way.
    pub fn replace_file(&mut self, file: usize, rows: Vec<DetailRow>) -> Option<Range<usize>> {
        let old_range = self.file_ranges.get(file)?.clone();
        let new_length = rows.len();
        self.rows.splice(old_range.clone(), rows);

        // Later files start where this one now ends.
        let shift = new_length.cast_signed() - old_range.len().cast_signed();
        self.file_ranges[file] = old_range.start..old_range.start + new_length;
        for range in &mut self.file_ranges[file + 1..] {
            range.start = range.start.saturating_add_signed(shift);
            range.end = range.end.saturating_add_signed(shift);
        }
        Some(old_range)
    }
}

/// The note a file shows in place of its lines, if any: a collapsed large
/// file offers to expand; an expanded file without lines says why.
fn file_note(
    loaded: &LoadedFile,
    state: FileDiffState,
    settings: DiffViewSettings,
) -> Option<FileNote> {
    if state.is_collapsed {
        let offers_expand = state.is_large && !state.is_viewed;
        return offers_expand.then(|| FileNote::CollapsedLarge {
            changed_lines: loaded.diff.changed_line_count(),
        });
    }
    // No patch: a move or copy without changes, else a binary or too large file.
    if loaded.file.patch.is_none() {
        let is_unchanged = loaded.file.additions == 0 && loaded.file.deletions == 0;
        return Some(match (loaded.file.status.as_str(), is_unchanged) {
            ("renamed", true) => FileNote::RenamedWithoutChanges,
            ("copied", true) => FileNote::CopiedWithoutChanges,
            (_, true | false) => FileNote::NoPatch,
        });
    }
    if !loaded.diff.is_empty() {
        return None;
    }
    Some(match (settings.whitespace, loaded.file.status.as_str()) {
        (WhitespaceMode::Ignore, _) => FileNote::OnlyWhitespace,
        (WhitespaceMode::Show, "renamed") => FileNote::RenamedWithoutChanges,
        (WhitespaceMode::Show, _) => FileNote::NoChanges,
    })
}

#[cfg(test)]
mod tests {
    use qc_diff::DiffOptions;
    use qc_github::PullRequestFile;

    use super::*;
    use crate::pull_request_detail::file_review_state::FileReviewState;

    const PATCH: &str = "@@ -1,2 +1,2 @@\n same\n-old\n+new\n";

    fn loaded(filename: &str, patch: Option<&str>) -> LoadedFile {
        LoadedFile::build(
            PullRequestFile {
                filename: filename.into(),
                previous_filename: None,
                status: "modified".into(),
                additions: 1,
                deletions: 1,
                patch: patch.map(Into::into),
            },
            DiffOptions::default(),
        )
    }

    fn build(
        files: &[LoadedFile],
        review: &FileReviewState,
        settings: DiffViewSettings,
    ) -> DetailRowList {
        let changed = ChangedFiles::Loaded(files.to_vec());
        let shown = vec![true; files.len()];
        DetailRowList::build(&DetailContent {
            has_pull_request: true,
            changed_files: Some(&changed),
            shown_files: &shown,
            file_review: review,
            settings,
        })
    }

    /// A unified file is its header, a hunk header, one row per line, and its end.
    #[test]
    fn builds_unified_rows() {
        let files = [loaded("a.rs", Some(PATCH))];
        let review = FileReviewState::for_loaded(&files, &[]);
        let list = build(&files, &review, DiffViewSettings::default());
        assert_eq!(
            list.rows,
            [
                DetailRow::SummaryBar,
                DetailRow::FileHeader { file: 0 },
                DetailRow::HunkHeader { file: 0, hunk: 0 },
                DetailRow::UnifiedLine {
                    file: 0,
                    hunk: 0,
                    line: 0
                },
                DetailRow::UnifiedLine {
                    file: 0,
                    hunk: 0,
                    line: 1
                },
                DetailRow::UnifiedLine {
                    file: 0,
                    hunk: 0,
                    line: 2
                },
                DetailRow::FileEnd { file: 0 },
            ]
        );
        assert_eq!(list.file_at_row(3), Some(0));
        assert_eq!(list.file_at_row(0), None);
    }

    /// Split rows pair the removed and added line into one row.
    #[test]
    fn builds_split_rows() {
        let files = [loaded("a.rs", Some(PATCH))];
        let review = FileReviewState::for_loaded(&files, &[]);
        let settings = DiffViewSettings {
            layout: DiffLayout::Split,
            ..DiffViewSettings::default()
        };
        let list = build(&files, &review, settings);
        let split_rows = list
            .rows
            .iter()
            .filter(|row| matches!(row, DetailRow::SplitLine { .. }))
            .count();
        assert_eq!(split_rows, 2);
    }

    /// Replacing a collapsed file's rows moves the files after it.
    #[test]
    fn replace_file_shifts_later_files() {
        let files = [loaded("a.rs", Some(PATCH)), loaded("b.rs", Some(PATCH))];
        let mut review = FileReviewState::for_loaded(&files, &[]);
        let mut list = build(&files, &review, DiffViewSettings::default());
        let second_start = list.file_start(1).unwrap();

        // Collapse the first file: only its header and end remain.
        review.toggle_collapsed("a.rs");
        let rows = DetailRowList::file_rows(
            0,
            &files[0],
            review.state_of(&files[0]),
            DiffViewSettings::default(),
        );
        let replaced = list.replace_file(0, rows).unwrap();

        assert_eq!(replaced.len(), 6);
        assert_eq!(list.file_start(1), Some(second_start - 4));
        assert_eq!(
            list.get(list.file_start(1).unwrap()),
            Some(DetailRow::FileHeader { file: 1 })
        );
    }

    /// Hidden files get no rows, and with all hidden the list says so.
    #[test]
    fn hidden_files_have_no_rows() {
        let files = [loaded("a.rs", Some(PATCH)), loaded("b.rs", Some(PATCH))];
        let review = FileReviewState::for_loaded(&files, &[]);
        let changed = ChangedFiles::Loaded(files.to_vec());
        let build = |shown: &[bool]| {
            DetailRowList::build(&DetailContent {
                has_pull_request: true,
                changed_files: Some(&changed),
                shown_files: shown,
                file_review: &review,
                settings: DiffViewSettings::default(),
            })
        };
        let first_hidden = build(&[false, true]);
        assert_eq!(first_hidden.file_start(0), None);
        assert_eq!(first_hidden.get(1), Some(DetailRow::FileHeader { file: 1 }));

        let all_hidden = build(&[false, false]);
        assert_eq!(
            all_hidden.rows,
            [DetailRow::SummaryBar, DetailRow::AllFilesHidden]
        );
    }

    /// A file moved without changes (GitHub sends no patch) says so, not "binary".
    #[test]
    fn moved_file_without_patch_shows_moved_note() {
        let mut file = (*loaded("new/name.rs", None).file).clone();
        file.status = "renamed".into();
        file.previous_filename = Some("old/name.rs".into());
        file.additions = 0;
        file.deletions = 0;
        let files = [LoadedFile::build(file, DiffOptions::default())];
        let review = FileReviewState::for_loaded(&files, &[]);
        let list = build(&files, &review, DiffViewSettings::default());
        assert!(list.rows.contains(&DetailRow::FileNote {
            file: 0,
            note: FileNote::RenamedWithoutChanges
        }));
    }

    /// A file without a patch shows a note instead of lines.
    #[test]
    fn binary_file_shows_note() {
        let files = [loaded("logo.png", None)];
        let review = FileReviewState::for_loaded(&files, &[]);
        let list = build(&files, &review, DiffViewSettings::default());
        assert!(list.rows.contains(&DetailRow::FileNote {
            file: 0,
            note: FileNote::NoPatch
        }));
    }
}
