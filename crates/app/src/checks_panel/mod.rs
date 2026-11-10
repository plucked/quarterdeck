//! The side panel's Checks tab: the CI checks on the shown pull request's
//! head commit, as a verdict, the checks that need attention, and every
//! check by workflow.
//!
//! Files:
//! - `checks_panel.rs`: `ChecksPanel`, the tab's view: what it shows, reading the checks and keeping them current.
//! - `shown_checks.rs`: `ShownChecks`, the checks as last read: arranged in a tree and where they came from, or only the rollup.
//! - `checks_target.rs`: `ChecksTarget`, the pull request the tab follows.
//! - `checks_rendering.rs`: the summary (verdict, counts, outcome bar, required checks) and the rows.
//! - `check_tree.rs`: `CheckTree`, the checks grouped by workflow and called workflow, in reading order.
//! - `check_tally.rs`: `CheckTally`, how many checks of a set stand at each outcome.
//! - `check_row.rs`: `CheckRow`, the list's rows, built from the tree, the folds and the skipped choice.
//! - `unfolded_checks.rs`: `UnfoldedChecks`, the workflows and groups the reader opened.
//! - `check_outcome_mark.rs`: `CheckOutcomeMark`, a check's outcome as a small round mark.

mod check_outcome_mark;
mod check_row;
mod check_tally;
mod check_tree;
mod checks_panel;
mod checks_rendering;
mod checks_target;
mod shown_checks;
mod unfolded_checks;

pub use checks_panel::ChecksPanel;
pub use checks_target::ChecksTarget;
