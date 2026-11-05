//! A file's unified patch, as GitHub sends it, turned into what a diff view
//! shows: hunks of numbered lines, side-by-side rows, and the changed words
//! or characters inside changed lines. Optionally treats whitespace-only
//! changes as unchanged. UI-free; the app's `diff_view` renders it.
//!
//! Files:
//! - `file_diff.rs`: `FileDiff`, the entry point: a file's hunks built with options.
//! - `diff_hunk.rs`: `DiffHunk`, one `@@` hunk, and its side-by-side rows.
//! - `diff_line.rs`: `DiffLine`, one numbered line with its changed ranges.
//! - `diff_line_kind.rs`: `DiffLineKind`, context, added or removed.
//! - `split_row.rs`: `SplitRow`, one side-by-side row as line indexes.
//! - `diff_options.rs`: `DiffOptions`, whitespace and highlight choices.
//! - `whitespace_mode.rs`: `WhitespaceMode`, show or ignore whitespace changes.
//! - `change_highlight.rs`: `ChangeHighlight`, lines, words or characters.
//! - `parse_patch.rs`: parses the patch text into hunks.
//! - `refine_changes.rs`: folds whitespace changes and marks inline changes.
//! - `change_block.rs`: `ChangeBlock`, a run of removed lines and the added lines replacing them.
//! - `inline_changes.rs`: `InlineChanges`, the differing parts of a removed/added pair.

mod change_block;
mod change_highlight;
mod diff_hunk;
mod diff_line;
mod diff_line_kind;
mod diff_options;
mod file_diff;
mod inline_changes;
mod parse_patch;
mod refine_changes;
mod split_row;
mod whitespace_mode;

pub use change_highlight::ChangeHighlight;
pub use diff_hunk::DiffHunk;
pub use diff_line::DiffLine;
pub use diff_line_kind::DiffLineKind;
pub use diff_options::DiffOptions;
pub use file_diff::FileDiff;
pub use split_row::SplitRow;
pub use whitespace_mode::WhitespaceMode;
