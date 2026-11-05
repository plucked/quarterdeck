//! The main view: the queue of pull requests that request the viewer's
//! review, with a detail pane for the selected one. It records its selection
//! and detail scroll as UI state and restores them after a relaunch.
//!
//! Files:
//! - `review_queue.rs`: `ReviewQueue`, the view's state, loading and `Render`.
//! - `refresh_status.rs`: `RefreshStatus`, the state of the queue's refresh.
//! - `refresh_schedule.rs`: `RefreshSchedule`, when the queue refreshes on its own.
//! - `activation_refresh.rs`: `ActivationRefresh`, whether a window activation refreshes.
//! - `changed_files.rs`: `ChangedFiles`, the load state of the selected pull request's files.
//! - `selection_after_refresh.rs`: decides what stays selected when the queue is replaced.
//! - `selection_restore.rs`: decides how a selection saved before a relaunch is restored.
//! - `restore_phase.rs`: `RestorePhase`, which queue a saved selection is matched against.
//! - `sidebar_rendering.rs`: renders the queue list and its header.
//! - `detail_rendering.rs`: renders the selected pull request.
//! - `render_file_diff.rs`: renders one changed file and its patch.

mod activation_refresh;
mod changed_files;
mod detail_rendering;
mod refresh_schedule;
mod refresh_status;
mod render_file_diff;
mod restore_phase;
mod review_queue;
mod selection_after_refresh;
mod selection_restore;
mod sidebar_rendering;

pub use review_queue::ReviewQueue;
