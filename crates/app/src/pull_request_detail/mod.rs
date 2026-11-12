//! The right pane: the shown pull request's header, description and changed
//! files as one virtualized list, so scrolling costs the same for a
//! one-line change as for a hundred files. For a stacked pull request it
//! shows one layer or several layers combined. Owns
//! loading the files (with progress), building and coloring their diffs, the
//! diff settings and each file's viewed and collapsed state; the review
//! (GitHub's conversations and the user's comments under their lines, and
//! submitting or copying them); the AI-written brief in place of the diff;
//! and the side panel's Files, Review, Actions (merge, close) and Checks tabs.
//!
//! Files:
//! - `pull_request_detail.rs`: `PullRequestDetail`, the view's state, what it shows and its accessors.
//! - `show_change.rs`: `ShowChange`, what showing a pull request changes after the one shown before.
//! - `file_loading.rs`: loading the review target's files, marks, notes and stored brief in the background.
//! - `syntax_highlighting.rs`: coloring the loaded diffs in batches, and applying changed diff settings.
//! - `highlighted_diff.rs`: `HighlightedDiff`, the syntax colors worked out for one diff.
//! - `row_rebuilding.rs`: building the rows again (all, or one file's) and keeping the scroll position.
//! - `scroll_anchor.rs`: `ScrollAnchor`, where the list scrolls after a rebuild.
//! - `stack_actions.rs`: the stack: loading its layers, the review mode, the combined range, moving between layers, the Checks tab's target.
//! - `stack_state.rs`: `StackState`, the shown pull request's stack: its layers, their load and error, the combined range's start.
//! - `file_navigation.rs`: the current file, moving between files, collapsing and marking them viewed, sideways scrolling.
//! - `file_direction.rs`: `FileDirection`, next or previous (file or stack layer).
//! - `file_tree_actions.rs`: the side panel and its file tree: showing it, the filter, folders, showing a file in it.
//! - `diff_selection_actions.rs`: selecting code in the diff with the mouse and copying it.
//! - `row_rendering.rs`: renders the pane's layout and each diff row.
//! - `row_height_estimate.rs`: where each row starts, estimated without layout, for the scrollbar.
//! - `sticky_header_rendering.rs`: the header above the diff: title, metadata, description.
//! - `sticky_file_header.rs`: pins the top file's header while its diff scrolls underneath.
//! - `github_link.rs`: `GitHubLink`, GitHub's mark as an icon button opening the pull request on GitHub.
//! - `side_panel_rendering.rs`: the side panel: its divider, its tabs and the shown tab.
//! - `file_tree_rendering.rs`: the side panel's Files tab: progress, folders and files.
//! - `filter_rendering.rs`: the filter: viewed switch, kind and type chip rows, summary.
//! - `detail_row.rs`: `DetailRow`, one row of the list.
//! - `detail_row_list.rs`: `DetailRowList`, the rows in order and each file's row range.
//! - `detail_content.rs`: `DetailContent`, what the row list is built from.
//! - `detail_event.rs`: `DetailEvent`, what the view tells its owner.
//! - `layer_request.rs`: `LayerRequest`, a stack layer to show, with its place in the stack.
//! - `diff_selection.rs`: `DiffSelection`, a text selection in one code column of the diff.
//! - `selected_diff_text.rs`: the code a selection covers, as copied.
//! - `selection_granularity.rs`: `SelectionGranularity`, what a press selects by its click count.
//! - `parsed_description.rs`: `ParsedDescription`, the description of one version, parsed once.
//! - `changed_rows.rs`: `ChangedRows`, the part of a file's rows a rebuild changed.
//! - `changed_files.rs`: `ChangedFiles`, the load state of the files.
//! - `fetch_changed_files.rs`: gets a pull request's files from the cache, asking GitHub as little as possible.
//! - `file_set_update.rs`: `FileSetUpdate`, whether cached files are confirmed or refetched.
//! - `loaded_file.rs`: `LoadedFile`, a changed file with its built diff, colors and sizes.
//! - `file_tree_state.rs`: `FileTreeState`, the file tree panel's tree, filter, folders and rows.
//! - `folder_viewed_state.rs`: `FolderViewedState`, none, some or all of a folder viewed.
//! - `folder_summary.rs`: `FolderSummary`, a folder's changed lines, file count, heat and viewed state for its tree row.
//! - `line_counts.rs`: `LineCounts`, lines added and removed.
//! - `file_review_state.rs`: `FileReviewState`, which files are viewed and collapsed.
//! - `viewed_toggle.rs`: `ViewedToggle`, what toggling a viewed checkbox did.
//! - `load_progress.rs`: `LoadProgress`, how far loading the files has come.
//! - `batch_progress.rs`: `BatchProgress`, how many of the syntax coloring's batches are done.
//! - `review_target.rs`: `ReviewTarget`, one layer or a combined range of stack layers.
//! - `fetch_combined_files.rs`: gets a combined range's files from the cache or one diff download.
//! - `stack_bar_rendering.rs`: the header's stack line: layer, neighbours, review mode, range.
//! - `review_pane.rs`: `ReviewPane`, the detail's review state: comments, cards, editor, tab.
//! - `review_comments.rs`: `ReviewComments`, GitHub's conversations and the user's notes, as cards.
//! - `comment_card.rs`: `CommentCard`, one card of comments: a GitHub conversation or a note.
//! - `placed_card.rs`: `PlacedCard`, a card with the file and line it goes under.
//! - `comment_placement.rs`: `CommentPlacement`, which line each card and the composer go under.
//! - `diff_line_ref.rs`: `DiffLineRef`, one line of a loaded file's diff, by file, side and number.
//! - `comment_range.rs`: `CommentRange`, the lines the comment being written is about.
//! - `comment_scope.rs`: `CommentScope`, whether GitHub's comments show with the local notes.
//! - `composer_opening.rs`: `ComposerOpening`, a new comment or stretching the open one.
//! - `comment_editor.rs`: `CommentEditor`, the comment being written and what it is for.
//! - `operation_status.rs`: `OperationStatus`, how the last review or pull request action went, and its line.
//! - `side_panel_tab.rs`: `SidePanelTab`, files, review, actions or checks.
//! - `review_actions.rs`: where new comments go, which cards show and where, starting the review over.
//! - `comment_editing.rs`: the comment editor: opening it on a line, a reply or an edit, saving and closing it.
//! - `github_review_actions.rs`: the GitHub review: loading the conversations, editing them on GitHub, submitting.
//! - `local_note_actions.rs`: the user's notes: adding, deleting, copying the local ones for an agent, finding one in the diff.
//! - `side_panel_actions.rs`: the side panel's tab, the review's verdict, summary and comment destination.
//! - `lines_label.rs`: `lines_label`, the lines a comment is about (`Line 42`, `Lines 40–42`).
//! - `local_review_export.rs`: the local notes as Markdown for an agent.
//! - `comment_card_rendering.rs`: the comment cards and the composer in the diff.
//! - `review_panel_rendering.rs`: the side panel's review tab.
//! - `destination_switch_rendering.rs`: the GitHub / Local switch of where new comments go.
//! - `thread_resolution.rs`: `ThreadResolution`, resolving or reopening a GitHub conversation.
//! - `action_panel.rs`: `ActionPanel`, the merge or close panel's options, choices and progress.
//! - `pull_request_actions.rs`: merging (also as admin), closing and deleting the branch on GitHub.
//! - `action_panel_rendering.rs`: the side panel's Actions tab: Merge… and Close…, and the open panel.
//! - `activity_rendering.rs`: the loading indicators: header pill, progress line, loading card.
//! - `context_expansion.rs`: revealing unchanged lines around hunks: fetching the file's text, rebuilding its rows.
//! - `fetch_file_text.rs`: gets a file's text after the change from the cache by blob SHA, else from GitHub.
//! - `head_commit.rs`: `HeadCommit`, the head commit the shown files are of, read from the cache when needed.
//! - `context_loads.rs`: `ContextLoads`, the file texts being fetched and their tasks.
//! - `detail_mode.rs`: `DetailMode`, the diff or the brief in the main column.
//! - `brief_pane.rs`: `BriefPane`, the briefs written this session, the run in flight, the brief's rows.
//! - `brief_key.rs`: `BriefKey`, which brief: a pull request and review target.
//! - `brief_row.rs`: `BriefRow`, one row of the brief's list.
//! - `run_cancel.rs`: `RunCancel`, kills the agent process when the run is dropped.
//! - `brief_job.rs`: `BriefJob`, one brief's background run: checkout, agent, checks, storing.
//! - `brief_code.rs`: `BriefCode`, the shallow checkout of the head commit a brief's agent reads (or why there is none).
//! - `smart_filter_state.rs`, `smart_filter_actions.rs`, `smart_filter_job.rs`, `smart_filter_rendering.rs`: saved review buckets, their agent run, filters and bulk viewed actions.
//! - `brief_actions.rs`: writing a brief with Claude Code and cancelling it, its pages and reader.
//! - `brief_handlers.rs`: `detail_brief_handlers`, what the brief's links and buttons do in the detail, and its report host.
//! - `studio_request.rs`: `studio_request`, what the agent is told about the shown pull request and its files.
//! - `change_diff_of.rs`: `change_diff_of`, the loaded patches as one unified diff for the agent's workspace.
//! - `file_lookup.rs`: `file_index_of`, the loaded file an anchor's path names; the loaded files as the brief's `BriefDiffs`.
//! - `brief_pane_rendering.rs`: the brief's rows in the main column, drawn by `brief_view`.

mod action_panel;
mod action_panel_rendering;
mod activity_rendering;
mod batch_progress;
mod brief_actions;
mod brief_code;
mod brief_handlers;
mod brief_job;
mod brief_key;
mod brief_pane;
mod brief_pane_rendering;
mod brief_row;
mod change_diff_of;
mod changed_files;
mod changed_rows;
mod comment_card;
mod comment_card_rendering;
mod comment_editing;
mod comment_editor;
mod comment_placement;
mod comment_range;
mod comment_scope;
mod composer_opening;
mod context_expansion;
mod context_loads;
mod destination_switch_rendering;
mod detail_content;
mod detail_event;
mod detail_mode;
mod detail_row;
mod detail_row_list;
mod diff_line_ref;
mod diff_selection;
mod diff_selection_actions;
mod fetch_changed_files;
mod fetch_combined_files;
mod fetch_file_text;
mod file_direction;
mod file_loading;
mod file_lookup;
mod file_navigation;
mod file_review_state;
mod file_set_update;
mod file_tree_actions;
mod file_tree_rendering;
mod file_tree_state;
mod filter_rendering;
mod folder_summary;
mod folder_viewed_state;
mod github_link;
mod github_review_actions;
mod head_commit;
mod highlighted_diff;
mod layer_request;
mod line_counts;
mod lines_label;
mod load_progress;
mod loaded_file;
mod local_note_actions;
mod local_review_export;
mod operation_status;
mod parsed_description;
mod placed_card;
mod pull_request_actions;
mod pull_request_detail;
mod review_actions;
mod review_comments;
mod review_pane;
mod review_panel_rendering;

mod review_target;
mod row_height_estimate;
mod row_rebuilding;
mod row_rendering;
mod run_cancel;
mod scroll_anchor;
mod selected_diff_text;
mod selection_granularity;
mod show_change;
mod side_panel_actions;
mod side_panel_rendering;
mod side_panel_tab;
mod smart_filter_actions;
mod smart_filter_job;
mod smart_filter_rendering;
mod smart_filter_state;
mod stack_actions;
mod stack_bar_rendering;
mod stack_state;
mod sticky_file_header;
mod sticky_header_rendering;
mod studio_request;
mod syntax_highlighting;
mod thread_resolution;
mod viewed_toggle;

pub use detail_event::DetailEvent;
pub use layer_request::LayerRequest;
pub use pull_request_detail::PullRequestDetail;
