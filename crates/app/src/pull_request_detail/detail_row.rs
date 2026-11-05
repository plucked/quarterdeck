use qc_diff::SplitRow;

use crate::diff_view::FileNote;

/// One row of the virtualized detail pane. Files, hunks and lines are
/// indexes into the loaded files, so a row is small and cheap to keep for
/// every line of a large pull request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailRow {
    /// No pull request is selected.
    NoSelection,
    /// Title and metadata of the pull request.
    Header,
    Description,
    FilesLoading,
    FilesFailed,
    /// The diff toolbar above the files.
    Toolbar,
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
}
