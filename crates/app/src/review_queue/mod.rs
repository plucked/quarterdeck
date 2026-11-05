//! The main view: the queue of pull requests that request the viewer's
//! review on the left, and the selected one in the detail pane on the
//! right. It records the selection and detail scroll as UI state and
//! restores them after a relaunch.
//!
//! Files:
//! - `review_queue.rs`: `ReviewQueue`, the queue, selection, refresh and restore, and `Render`.
//! - `sidebar_rendering.rs`: renders the queue list and its header.
//! - `refresh_status.rs`: `RefreshStatus`, the state of the queue's refresh.
//! - `refresh_schedule.rs`: `RefreshSchedule`, when the queue refreshes on its own.
//! - `activation_refresh.rs`: `ActivationRefresh`, whether a window activation refreshes.
//! - `selection_after_refresh.rs`: decides what stays selected when the queue is replaced.
//! - `selection_restore.rs`: decides how a selection saved before a relaunch is restored.
//! - `restore_phase.rs`: `RestorePhase`, which queue a saved selection is matched against.

mod activation_refresh;
mod refresh_schedule;
mod refresh_status;
mod restore_phase;
mod review_queue;
mod selection_after_refresh;
mod selection_restore;
mod sidebar_rendering;

pub use review_queue::ReviewQueue;
