//! Renders file diffs built by `qc-diff`: a reusable `FileDiffView` per file
//! (header with status, path, counts, viewed checkbox; unified or
//! side-by-side body with line numbers and inline change marks; wrapped or
//! scrolling lines) and the `DiffToolbar` that switches how they show.
//!
//! Files:
//! - `file_diff_view.rs`: `FileDiffView`, one file's header and body.
//! - `file_diff_state.rs`: `FileDiffState`, whether a file is collapsed, viewed or large.
//! - `file_header_rendering.rs`: renders a file's header row.
//! - `unified_rendering.rs`: renders a body as one column.
//! - `split_rendering.rs`: renders a body as old and new side by side.
//! - `diff_cell_rendering.rs`: the shared pieces: gutters, markers, code, hunk headers, scroll bodies.
//! - `diff_toolbar.rs`: `DiffToolbar`, the summary and the view options.
//! - `diff_summary.rs`: `DiffSummary`, file, line and viewed counts.

mod diff_cell_rendering;
mod diff_summary;
mod diff_toolbar;
mod file_diff_state;
mod file_diff_view;
mod file_header_rendering;
mod split_rendering;
mod unified_rendering;

pub use diff_summary::DiffSummary;
pub use diff_toolbar::DiffToolbar;
pub use file_diff_state::FileDiffState;
pub use file_diff_view::FileDiffView;
