//! The main view: the viewer's own pull requests and those that request
//! their review on the left (a tab each, stacked pull requests grouped by
//! stack), and the selected one in the detail pane on the right. It opens
//! stack layers the detail asks for, and records the tab, selection and
//! detail scroll as UI state to restore them after a relaunch.
//!
//! Files:
//! - `review_queue.rs`: `ReviewQueue`, the queue, selection, refresh and restore, and `Render`.
//! - `sidebar_rendering.rs`: renders the queue list and its status line.
//! - `top_bar_rendering.rs`: the bar across the window's top (panel toggles, tabs, refresh).
//! - `refresh_status.rs`: `RefreshStatus`, the state of the queue's refresh.
//! - `refresh_schedule.rs`: `RefreshSchedule`, when the queue refreshes on its own.
//! - `activation_refresh.rs`: `ActivationRefresh`, whether a window activation refreshes.
//! - `selection_after_refresh.rs`: decides what stays selected when the queue is replaced.
//! - `selection_restore.rs`: decides how a selection saved before a relaunch is restored.
//! - `restore_phase.rs`: `RestorePhase`, which queue a saved selection is matched against.
//! - `queue_lists.rs`: `QueueLists`, the authored and review-requested lists.
//! - `queue_row.rs`: `QueueRow`, a sidebar row (pull request, stack header, stack layer), and grouping.
//! - `stack_key.rs`: `StackKey`, a stack's repository and number.
//! - `selection_origin.rs`: `SelectionOrigin`, picked from a list or opened from a stack.

mod activation_refresh;
mod queue_lists;
mod queue_row;
mod refresh_schedule;
mod refresh_status;
mod restore_phase;
mod review_queue;
mod selection_after_refresh;
mod selection_origin;
mod selection_restore;
mod sidebar_rendering;
mod stack_key;
mod top_bar_rendering;

pub use review_queue::ReviewQueue;
