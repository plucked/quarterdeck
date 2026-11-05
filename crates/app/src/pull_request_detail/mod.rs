//! The right pane: the shown pull request's header, description and changed
//! files as one virtualized list, so scrolling costs the same for a
//! one-line change as for a hundred files. For a stacked pull request it
//! shows one layer or several layers combined, with the stack map. Owns
//! loading the files (with progress), building and coloring their diffs, the
//! diff settings and each file's viewed and collapsed state.
//!
//! Files:
//! - `pull_request_detail.rs`: `PullRequestDetail`, the view's state, loading and actions.
//! - `row_rendering.rs`: renders the pane's layout and each diff row.
//! - `sticky_header_rendering.rs`: the header above the diff: title, metadata, description.
//! - `file_tree_rendering.rs`: the file tree panel: progress, folders and files.
//! - `filter_rendering.rs`: the filter: viewed switch, kind and type chip rows, summary.
//! - `detail_row.rs`: `DetailRow`, one row of the list.
//! - `detail_row_list.rs`: `DetailRowList`, the rows in order and each file's row range.
//! - `detail_content.rs`: `DetailContent`, what the row list is built from.
//! - `detail_event.rs`: `DetailEvent`, what the view tells its owner.
//! - `diff_selection.rs`: `DiffSelection`, a text selection in one code column of the diff.
//! - `selected_diff_text.rs`: the code a selection covers, as copied.
//! - `parsed_description.rs`: `ParsedDescription`, the description of one version, parsed once.
//! - `changed_rows.rs`: `ChangedRows`, the part of a file's rows a rebuild changed.
//! - `changed_files.rs`: `ChangedFiles`, the load state of the files.
//! - `fetch_changed_files.rs`: gets a pull request's files from the cache, asking GitHub as little as possible.
//! - `file_set_update.rs`: `FileSetUpdate`, whether cached files are confirmed or refetched.
//! - `loaded_file.rs`: `LoadedFile`, a changed file with its built diff, colors and sizes.
//! - `file_tree_state.rs`: `FileTreeState`, the file tree panel's tree, filter, folders and rows.
//! - `folder_viewed_state.rs`: `FolderViewedState`, none, some or all of a folder viewed.
//! - `file_review_state.rs`: `FileReviewState`, which files are viewed and collapsed.
//! - `viewed_toggle.rs`: `ViewedToggle`, what toggling a viewed checkbox did.
//! - `chip_click.rs`: `ChipClick`, a filter chip clicked to pick or (option) to hide.
//! - `load_progress.rs`: `LoadProgress`, how far loading the files has come.
//! - `review_target.rs`: `ReviewTarget`, one layer or a combined range of stack layers.
//! - `fetch_combined_files.rs`: gets a combined range's files from the cache or one diff download.
//! - `stack_bar_rendering.rs`: the header's stack line: layer, neighbours, review mode, range.
//! - `stack_map_rendering.rs`: the stack map above the file tree: every layer, the range.
//! - `review_pane.rs`: `ReviewPane`, the detail's review state: comments, cards, editor, tab.
//! - `review_comments.rs`: `ReviewComments`, GitHub's conversations and the user's notes, as cards.
//! - `comment_placement.rs`: `CommentPlacement`, which line each card and the composer go under.
//! - `comment_editor.rs`: `CommentEditor`, the comment being written and what it is for.
//! - `review_status.rs`: `ReviewStatus`, how the last review action went.
//! - `side_panel_tab.rs`: `SidePanelTab`, files or review.
//! - `review_actions.rs`: writing, editing, deleting and submitting comments; loading GitHub's.
//! - `local_review_export.rs`: the local notes as Markdown for an agent.
//! - `comment_card_rendering.rs`: the comment cards and the composer in the diff.
//! - `review_panel_rendering.rs`: the side panel's review tab.
//! - `action_panel.rs`: `ActionPanel`, the merge or close panel's options, choices and progress.
//! - `pull_request_actions.rs`: merging (also as admin), closing and deleting the branch on GitHub.
//! - `action_panel_rendering.rs`: the merge or close panel under the header.
//! - `activity_rendering.rs`: the loading indicators: header pill, progress line, loading card.

mod action_panel;
mod action_panel_rendering;
mod activity_rendering;
mod changed_files;
mod changed_rows;
mod chip_click;
mod comment_card_rendering;
mod comment_editor;
mod comment_placement;
mod detail_content;
mod detail_event;
mod detail_row;
mod detail_row_list;
mod diff_selection;
mod fetch_changed_files;
mod fetch_combined_files;
mod file_review_state;
mod file_set_update;
mod file_tree_rendering;
mod file_tree_state;
mod filter_rendering;
mod folder_viewed_state;
mod load_progress;
mod loaded_file;
mod local_review_export;
mod parsed_description;
mod pull_request_actions;
mod pull_request_detail;
mod review_actions;
mod review_comments;
mod review_pane;
mod review_panel_rendering;
mod review_status;
mod review_target;
mod row_rendering;
mod selected_diff_text;
mod side_panel_tab;
mod stack_bar_rendering;
mod stack_map_rendering;
mod sticky_header_rendering;
mod viewed_toggle;

pub use detail_event::{DetailEvent, LayerRequest};
pub use pull_request_detail::PullRequestDetail;
