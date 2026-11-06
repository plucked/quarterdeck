//! Renders file diffs built by `qd-diff` row by row, for the virtualized
//! detail list: file headers (status, path, counts, viewed checkbox), hunk
//! headers, unified lines or side-by-side rows with line numbers, syntax
//! colors and inline change marks, bars revealing the unchanged lines
//! between hunks, and notes for files without lines. Each
//! row draws its piece of the file's card. Also the `DiffSummaryBar` above
//! the files; how diffs show is chosen in the settings window.
//!
//! Files:
//! - `file_header_rendering.rs`: a file's header row, the top of its card.
//! - `file_status_badge.rs`: the letter badge for a file's status (added, removed, renamed…).
//! - `unified_rendering.rs`: one unified diff row.
//! - `split_rendering.rs`: one side-by-side diff row.
//! - `diff_cell_rendering.rs`: the shared pieces: rows, gutters, markers, code, hunk headers.
//! - `code_highlights.rs`: merges a line's syntax colors with its changed ranges.
//! - `file_card_rendering.rs`: the card's side borders and closing strip.
//! - `file_note_rendering.rs`: a file's note row.
//! - `file_note.rs`: `FileNote`, why a file shows a note.
//! - `file_diff_state.rs`: `FileDiffState`, whether a file is collapsed, viewed or large.
//! - `diff_row_style.rs`: `DiffRowStyle`, what a file's rows share.
//! - `diff_side.rs`: `DiffSide`, the code column of a row (unified, old, new).
//! - `diff_position.rs`: `DiffPosition`, a row and byte in the diff text.
//! - `code_segments.rs`: `CodeSegments`, the code texts painted this frame, for hit testing.
//! - `code_cells.rs`: `CodeCells` and `SideSelection`, what a row's code cells need for selection.
//! - `diff_summary_bar.rs`: `DiffSummaryBar`, totals and collapse/expand all.
//! - `diff_summary.rs`: `DiffSummary`, file and line counts.
//! - `context_gap_rendering.rs`: the bar over hidden unchanged lines, with buttons revealing them.
//! - `gap_part.rs`: `GapPart`, which lines of a gap a click reveals.
//! - `file_text_load.rs`: `FileTextLoad`, a file's text being fetched for its gaps, or failed.

mod code_cells;
mod code_highlights;
mod code_segments;
mod context_gap_rendering;
mod diff_cell_rendering;
mod diff_position;
mod diff_row_style;
mod diff_side;
mod diff_summary;
mod diff_summary_bar;
mod file_card_rendering;
mod file_diff_state;
mod file_header_rendering;
mod file_note;
mod file_note_rendering;
mod file_status_badge;
mod file_text_load;
mod gap_part;
mod split_rendering;
mod unified_rendering;

pub use code_cells::{CodeCells, SideSelection};
pub use code_segments::{CodeSegment, CodeSegments};
pub use context_gap_rendering::{GapClickHandler, render_gap_bar};
pub use diff_cell_rendering::{
    DIFF_CHARACTER_WIDTH, DIFF_ROW_HEIGHT, gutter_width, render_hunk_header,
};
pub use diff_position::DiffPosition;
pub use diff_row_style::DiffRowStyle;
pub use diff_side::DiffSide;
pub use diff_summary::DiffSummary;
pub use diff_summary_bar::DiffSummaryBar;
pub use file_card_rendering::{frame_file_row, render_file_end};
pub use file_diff_state::FileDiffState;
pub use file_header_rendering::{FileClickHandler, render_file_header};
pub use file_note::FileNote;
pub use file_note_rendering::render_file_note;
pub use file_status_badge::{FILE_STATUS_BADGE_SIZE, render_file_status_badge};
pub use file_text_load::FileTextLoad;
pub use gap_part::{CONTEXT_STEP, GapPart};
pub use split_rendering::render_split_row;
pub use unified_rendering::render_unified_line;
