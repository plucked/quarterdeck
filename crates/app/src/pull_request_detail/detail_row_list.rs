use std::collections::HashSet;
use std::ops::Range;

use qd_cache::{DiffLayout, DiffViewSettings};
use qd_diff::{GapPlace, WhitespaceMode};
use qd_github::LineSide;

use super::changed_files::ChangedFiles;
use super::comment_placement::CommentPlacement;
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
                        list.rows.extend(Self::file_rows(
                            index,
                            loaded,
                            state,
                            content.settings,
                            content.comments,
                        ));
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
    /// the bar over hidden lines below its last hunk (when it has some),
    /// then the strip closing its card.
    pub fn file_rows(
        file: usize,
        loaded: &LoadedFile,
        state: FileDiffState,
        settings: DiffViewSettings,
        comments: &CommentPlacement,
    ) -> Vec<DetailRow> {
        let mut rows = vec![DetailRow::FileHeader { file }];
        if let Some(note) = file_note(loaded, state, settings) {
            rows.push(DetailRow::FileNote { file, note });
        } else if !state.is_collapsed {
            let mut placed: HashSet<usize> = HashSet::new();
            for (hunk_index, hunk) in loaded.diff.hunks().iter().enumerate() {
                rows.push(DetailRow::HunkHeader {
                    file,
                    hunk: hunk_index,
                });
                match settings.layout {
                    DiffLayout::Unified => {
                        for (line_index, line) in hunk.lines.iter().enumerate() {
                            rows.push(DetailRow::UnifiedLine {
                                file,
                                hunk: hunk_index,
                                line: line_index,
                            });
                            let anchors = [
                                line.new_number.map(|number| (LineSide::New, number)),
                                line.old_number.map(|number| (LineSide::Old, number)),
                            ];
                            push_comment_rows(&mut rows, file, &anchors, comments, &mut placed);
                        }
                    }
                    DiffLayout::Split => {
                        for row in hunk.split_rows() {
                            rows.push(DetailRow::SplitLine {
                                file,
                                hunk: hunk_index,
                                row,
                            });
                            let line_number = |index: Option<usize>, side: LineSide| {
                                let line = hunk.lines.get(index?)?;
                                let number = match side {
                                    LineSide::Old => line.old_number,
                                    LineSide::New => line.new_number,
                                }?;
                                Some((side, number))
                            };
                            let anchors = [
                                line_number(row.new, LineSide::New),
                                line_number(row.old, LineSide::Old),
                            ];
                            push_comment_rows(&mut rows, file, &anchors, comments, &mut placed);
                        }
                    }
                }
            }

            // The lines below the last hunk, when the diff leaves some out.
            if loaded.gap_at(GapPlace::End).is_some() {
                rows.push(DetailRow::EndGap { file });
            }

            // Cards no line took (outdated conversations, lines not in the diff) go under the header.
            let unplaced: Vec<DetailRow> = comments
                .cards_of(file)
                .iter()
                .filter(|card| !placed.contains(card))
                .map(|card| DetailRow::CommentCard { file, card: *card })
                .collect();
            rows.splice(1..1, unplaced);
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

    /// The rows in `range`.
    pub fn rows_in(&self, range: Range<usize>) -> &[DetailRow] {
        self.rows.get(range).unwrap_or_default()
    }

    /// The rows of file `file`; empty while it is hidden.
    pub fn file_range(&self, file: usize) -> Option<Range<usize>> {
        self.file_ranges.get(file).cloned()
    }

    /// The first row (the header) of file `file`; `None` while it is hidden.
    pub fn file_start(&self, file: usize) -> Option<usize> {
        self.file_ranges
            .get(file)
            .filter(|range| !range.is_empty())
            .map(|range| range.start)
    }

    /// The file whose rows include row `row`; `None` above the files. The
    /// ranges follow each other in order, so this is a binary search (the
    /// tree asks on every frame, and a pull request can have 17k files).
    pub fn file_at_row(&self, row: usize) -> Option<usize> {
        let file = self.file_ranges.partition_point(|range| range.end <= row);
        self.file_ranges
            .get(file)
            .filter(|range| range.contains(&row))
            .map(|_| file)
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

/// Pushes the comment cards under a diff row's lines (`anchors`), then the
/// composer when it is open on one of them, recording the cards placed.
fn push_comment_rows(
    rows: &mut Vec<DetailRow>,
    file: usize,
    anchors: &[Option<(LineSide, u32)>],
    comments: &CommentPlacement,
    placed: &mut HashSet<usize>,
) {
    for (side, line) in anchors.iter().flatten() {
        for card in comments.cards_on(file, *side, *line) {
            if placed.insert(*card) {
                rows.push(DetailRow::CommentCard { file, card: *card });
            }
        }
    }
    let has_composer = anchors
        .iter()
        .flatten()
        .any(|(side, line)| comments.has_composer_on(file, *side, *line));
    if has_composer {
        rows.push(DetailRow::Composer { file });
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
        return Some(
            match (loaded.file.status.as_str(), loaded.is_unchanged_move()) {
                ("renamed", true) => FileNote::RenamedWithoutChanges,
                ("copied", true) => FileNote::CopiedWithoutChanges,
                (_, true | false) => FileNote::NoPatch,
            },
        );
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
    use qd_diff::DiffOptions;
    use qd_github::PullRequestFile;

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
                sha: None,
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
            comments: &CommentPlacement::default(),
        })
    }

    /// A unified file is its header, a hunk header, one row per line, the
    /// bar over the lines below its hunk, and its end.
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
                DetailRow::EndGap { file: 0 },
                DetailRow::FileEnd { file: 0 },
            ]
        );
        assert_eq!(list.file_at_row(3), Some(0));
        assert_eq!(list.file_at_row(0), None);
    }

    /// The file at a row is found past hidden files' empty ranges, and
    /// nothing above the files or past the end.
    #[test]
    fn finds_the_file_at_a_row_past_hidden_files() {
        let list = DetailRowList {
            rows: vec![DetailRow::SummaryBar; 9],
            file_ranges: vec![1..5, 5..5, 5..9],
        };
        assert_eq!(list.file_at_row(0), None);
        assert_eq!(list.file_at_row(1), Some(0));
        assert_eq!(list.file_at_row(4), Some(0));
        assert_eq!(list.file_at_row(5), Some(2));
        assert_eq!(list.file_at_row(8), Some(2));
        assert_eq!(list.file_at_row(9), None);
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
            &CommentPlacement::default(),
        );
        let replaced = list.replace_file(0, rows).unwrap();

        assert_eq!(replaced.len(), 7);
        assert_eq!(list.file_start(1), Some(second_start - 5));
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
                comments: &CommentPlacement::default(),
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

    /// An added file shows whole, so it has no bar over hidden lines below it.
    #[test]
    fn added_file_has_no_end_gap() {
        let mut file = (*loaded("new.rs", Some("@@ -0,0 +1,2 @@\n+a\n+b\n")).file).clone();
        file.status = "added".into();
        let files = [LoadedFile::build(file, DiffOptions::default())];
        let review = FileReviewState::for_loaded(&files, &[]);
        let list = build(&files, &review, DiffViewSettings::default());
        assert!(!list.rows.contains(&DetailRow::EndGap { file: 0 }));
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
