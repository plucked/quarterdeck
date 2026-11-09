//! The right pane: the shown pull request's header, description and changed
//! files as one virtualized list, so scrolling costs the same for a
//! one-line change as for a hundred files. For a stacked pull request it
//! shows one layer or several layers combined. Owns
//! loading the files (with progress), building and coloring their diffs, the
//! diff settings and each file's viewed and collapsed state.
//!
//! Files:
//! - `pull_request_detail.rs`: `PullRequestDetail`, the view's state, loading and actions.
//! - `row_rendering.rs`: renders the pane's layout and each diff row.
//! - `row_height_estimate.rs`: where each row starts, estimated without layout, for the scrollbar.
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
//! - `folder_summary.rs`: `FolderSummary`, a folder's changed lines and viewed state for its tree row.
//! - `line_counts.rs`: `LineCounts`, lines added and removed.
//! - `file_review_state.rs`: `FileReviewState`, which files are viewed and collapsed.
//! - `viewed_toggle.rs`: `ViewedToggle`, what toggling a viewed checkbox did.
//! - `load_progress.rs`: `LoadProgress`, how far loading the files has come.
//! - `review_target.rs`: `ReviewTarget`, one layer or a combined range of stack layers.
//! - `fetch_combined_files.rs`: gets a combined range's files from the cache or one diff download.
//! - `stack_bar_rendering.rs`: the header's stack line: layer, neighbours, review mode, range.
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
//! - `context_expansion.rs`: revealing unchanged lines around hunks: fetching the file's text, rebuilding its rows.
//! - `fetch_file_text.rs`: gets a file's text after the change from the cache by blob SHA, else from GitHub.
//! - `context_loads.rs`: `ContextLoads`, the file texts being fetched and their tasks.
//! - `detail_mode.rs`: `DetailMode`, the diff or the brief in the main column.
//! - `brief_pane.rs`: `BriefPane`, the briefs written this session, the run in flight, the brief's rows.
//! - `brief_row.rs`: `BriefRow`, one row of the brief's list.
//! - `run_cancel.rs`: `RunCancel`, kills the agent process when the run is dropped.
//! - `brief_job.rs`: `BriefJob`, one brief's background run: checkout, agent, checks, storing.
//! - `brief_code.rs`: `BriefCode`, the shallow checkout of the head commit a brief's agent reads (or why there is none).
//! - `brief_actions.rs`: writing a brief with Claude Code, cancelling it, and opening an anchor in the diff.
//! - `file_lookup.rs`: `file_index_of`, the loaded file an anchor's path names; the loaded files as the brief's `BriefDiffs`.
//! - `brief_pane_rendering.rs`: the brief's rows in the main column, drawn by `brief_view`.

mod action_panel;
mod action_panel_rendering;
mod activity_rendering;
mod brief_actions;
mod brief_code;
mod brief_job;
mod brief_pane;
mod brief_pane_rendering;
mod brief_row;
mod changed_files;
mod changed_rows;
mod comment_card_rendering;
mod comment_editor;
mod comment_placement;
mod context_expansion;
mod context_loads;
mod detail_content;
mod detail_event;
mod detail_mode;
mod detail_row;
mod detail_row_list;
mod diff_selection;
mod fetch_changed_files;
mod fetch_combined_files;
mod fetch_file_text;
mod file_lookup;
mod file_review_state;
mod file_set_update;
mod file_tree_rendering;
mod file_tree_state;
mod filter_rendering;
mod folder_summary;
mod folder_viewed_state;
mod line_counts;
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
mod row_height_estimate;
mod row_rendering;
mod run_cancel;
mod selected_diff_text;
mod side_panel_tab;
mod stack_bar_rendering;
mod sticky_header_rendering;
mod viewed_toggle;

pub use detail_event::{DetailEvent, LayerRequest};
pub use pull_request_detail::PullRequestDetail;
