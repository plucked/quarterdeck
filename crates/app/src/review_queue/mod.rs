//! The main view: the viewer's own pull requests, those that request
//! their review or that they reviewed, and others' in watched repositories
//! on the left (Mine, Review and Others tabs, grouped by repository and stack), and the selected one in the
//! detail pane on the right. It opens
//! stack layers the detail asks for, and records the tab, selection and
//! detail scroll as UI state to restore them after a relaunch.
//!
//! Files:
//! - `review_queue.rs`: `ReviewQueue`, the queue, selection, refresh and restore, and `Render`.
//! - `sidebar_rendering.rs`: renders the queue list, its tabs and its status line.
//! - `top_bar_rendering.rs`: the bar across the window's top (panel toggles, refresh, title).
//! - `refresh_status.rs`: `RefreshStatus`, the state of the queue's refresh.
//! - `refresh_schedule.rs`: `RefreshSchedule`, when the queue refreshes on its own.
//! - `activation_refresh.rs`: `ActivationRefresh`, whether a window activation refreshes.
//! - `selection_after_refresh.rs`: decides what stays selected when the queue is replaced.
//! - `selection_restore.rs`: decides how a selection saved before a relaunch is restored.
//! - `restore_phase.rs`: `RestorePhase`, which queue a saved selection is matched against.
//! - `queue_lists.rs`: `QueueLists`, the authored, review-requested, reviewed and others' lists.
//! - `hidden_branches.rs`: whether the hidden branch rules hide a pull request, and the lists without those.
//! - `others_tab.rs`: `OthersTab`, the Others tab's state: watched repositories, their filters, what passes, their order.
//! - `others_tab_actions.rs`: the queue's Others actions: open a filter, change it, follow the watched repositories.
//! - `others_filtering.rs`: whether a pull request passes an Others filter, and the chips' counts.
//! - `others_facet.rs`: `OthersFacet`, an Others filter's chip row (labels or authors) and its part of the filter.
//! - `others_filter_rendering.rs`: renders a repository's Others filter under its header.
//! - `open_filter_chips.rs`: `OpenFilterChips`, the open Others filter's chip rows, counted ahead of drawing.
//! - `chip_marks.rs`: `ChipMarks`, what an Others chip shows before its name (an avatar, a label color).
//! - `queue_row.rs`: `QueueRow`, a sidebar row (repository header, pull request, stack header, stack layer), and grouping.
//! - `repo_group.rs`: `RepoGroup`, what a repository's header shows.
//! - `stack_group.rs`: `StackGroup`, what a stack's header shows.
//! - `others_group.rs`: `OthersGroup`, an Others repository's count before its filter, and the filter's state.
//! - `others_repository.rs`: `OthersRepository`, a repository the Others tab shows, with its `OthersGroup`.
//! - `collapsed_groups.rs`: `CollapsedGroups`, the repositories and stacks folded in the sidebar.
//! - `stack_key.rs`: `StackKey`, a stack's repository and number.
//! - `selection_origin.rs`: `SelectionOrigin`, picked from a list or opened from a stack.

mod activation_refresh;
mod chip_marks;
mod collapsed_groups;
mod hidden_branches;
mod open_filter_chips;
mod others_facet;
mod others_filter_rendering;
mod others_filtering;
mod others_group;
mod others_repository;
mod others_tab;
mod others_tab_actions;
mod queue_lists;
mod queue_row;
mod refresh_schedule;
mod refresh_status;
mod repo_group;
mod restore_phase;
mod review_queue;
mod selection_after_refresh;
mod selection_origin;
mod selection_restore;
mod sidebar_rendering;
mod stack_group;
mod stack_key;
mod top_bar_rendering;

pub use review_queue::ReviewQueue;
