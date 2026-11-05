//! The right pane: the shown pull request's header, description and changed
//! files as one virtualized list, so scrolling costs the same for a
//! one-line change as for a hundred files. Owns loading the files, building
//! and coloring their diffs, the diff settings and each file's viewed and
//! collapsed state.
//!
//! Files:
//! - `pull_request_detail.rs`: `PullRequestDetail`, the view's state, loading and actions.
//! - `row_rendering.rs`: renders the pane's layout and each diff row.
//! - `sticky_header_rendering.rs`: the header above the diff: title, metadata, description.
//! - `file_tree_rendering.rs`: the file tree panel: progress, filter chips, folders and files.
//! - `detail_row.rs`: `DetailRow`, one row of the list.
//! - `detail_row_list.rs`: `DetailRowList`, the rows in order and each file's row range.
//! - `detail_content.rs`: `DetailContent`, what the row list is built from.
//! - `detail_event.rs`: `DetailEvent`, what the view tells its owner.
//! - `diff_selection.rs`: `DiffSelection`, a text selection in one code column of the diff.
//! - `selected_diff_text.rs`: the code a selection covers, as copied.
//! - `parsed_description.rs`: `ParsedDescription`, the description of one version, parsed once.
//! - `changed_files.rs`: `ChangedFiles`, the load state of the files.
//! - `fetch_changed_files.rs`: gets a pull request's files from the cache, asking GitHub as little as possible.
//! - `file_set_update.rs`: `FileSetUpdate`, whether cached files are confirmed or refetched.
//! - `loaded_file.rs`: `LoadedFile`, a changed file with its built diff, colors and sizes.
//! - `file_tree_state.rs`: `FileTreeState`, the file tree panel's tree, filter, folders and rows.
//! - `filter_toggle.rs`: `FilterToggle`, one hide switch of the file filter.
//! - `folder_viewed_state.rs`: `FolderViewedState`, none, some or all of a folder viewed.
//! - `file_review_state.rs`: `FileReviewState`, which files are viewed and collapsed.
//! - `viewed_toggle.rs`: `ViewedToggle`, what toggling a viewed checkbox did.

mod changed_files;
mod detail_content;
mod detail_event;
mod detail_row;
mod detail_row_list;
mod diff_selection;
mod fetch_changed_files;
mod file_review_state;
mod file_set_update;
mod file_tree_rendering;
mod file_tree_state;
mod filter_toggle;
mod folder_viewed_state;
mod loaded_file;
mod parsed_description;
mod pull_request_detail;
mod row_rendering;
mod selected_diff_text;
mod sticky_header_rendering;
mod viewed_toggle;

pub use detail_event::DetailEvent;
pub use pull_request_detail::PullRequestDetail;
