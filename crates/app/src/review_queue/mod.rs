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
//! - `loaded_file.rs`: `LoadedFile`, a changed file with its built diff and fingerprint.
//! - `file_review_state.rs`: `FileReviewState`, which files are viewed and collapsed.
//! - `viewed_toggle.rs`: `ViewedToggle`, what toggling a viewed checkbox did.

mod activation_refresh;
mod changed_files;
mod detail_rendering;
mod file_review_state;
mod loaded_file;
mod refresh_schedule;
mod refresh_status;
mod restore_phase;
mod review_queue;
mod selection_after_refresh;
mod selection_restore;
mod sidebar_rendering;
mod viewed_toggle;

pub use review_queue::ReviewQueue;
