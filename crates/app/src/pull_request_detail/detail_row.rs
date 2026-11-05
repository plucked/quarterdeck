use qd_diff::SplitRow;

use crate::diff_view::FileNote;

/// One row of the virtualized detail pane. Files, hunks and lines are
/// indexes into the loaded files, so a row is small and cheap to keep for
/// every line of a large pull request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailRow {
    /// No pull request is selected.
    NoSelection,
    FilesLoading,
    FilesFailed,
    /// The totals above the files.
    SummaryBar,
    /// Every file is hidden by the filters (or viewed while viewed files are hidden).
    AllFilesHidden,
    FileHeader {
        file: usize,
    },
    FileNote {
        file: usize,
        note: FileNote,
    },
    HunkHeader {
        file: usize,
        hunk: usize,
    },
    UnifiedLine {
        file: usize,
        hunk: usize,
        line: usize,
    },
    SplitLine {
        file: usize,
        hunk: usize,
        row: SplitRow,
    },
    /// The strip closing a file's card.
    FileEnd {
        file: usize,
    },
    /// A card of review comments in file `file` (an index into the detail's cards).
    CommentCard {
        file: usize,
        card: usize,
    },
    /// The editor of a new comment, under its line in file `file`.
    Composer {
        file: usize,
    },
}
