//! The app's local SQLite database: cached GitHub data, so the app shows the
//! last known state instantly and works offline, and the UI state it
//! restores after a relaunch. All state is stored in typed columns.
//!
//! Files:
//! - `github_cache.rs`: `GitHubCache`, the connection to the cached GitHub data.
//! - `pull_request_storage.rs`: cached pull requests and lists, with their labels and row mapping.
//! - `file_set_storage.rs`: a pull request's cached changed files, and confirming them.
//! - `cache_cleanup.rs`: removing expired cache entries, and clearing the cache.
//! - `cache_usage.rs`: `CacheUsage` and `CachedData`, how much the cache holds and takes on disk.
//! - `cache_usage_storage.rs`: reading the cache's usage.
//! - `cached_file_set.rs`: `CachedFileSet`, cached files with what they were fetched for.
//! - `file_set_revision.rs`: `FileSetRevision`, the commits and response tag of a file set.
//! - `cleanup_counts.rs`: `CleanupCounts`, what a cleanup removed.
//! - `cache_ttl.rs`: `CacheTtl`, how long unused cached data is kept.
//! - `cache_settings_storage.rs`: reads and writes of the cache TTL.
//! - `color_theme.rs`: `ColorTheme`, the app's color theme.
//! - `appearance_settings_storage.rs`: reads and writes of the color theme.
//! - `cached_statements.rs`: `execute_cached`, a write through the connection's statement cache, for statements run per row.
//! - `sql_integers.rs`: checked conversions between GitHub numbers and SQLite integers.
//! - `unix_now.rs`: the current time as stored.
//! - `ui_state_store.rs`: `UiStateStore`, the UI state connection; each topic's reads and writes sit in its `…_storage.rs` file.
//! - `window_placement_storage.rs`: reads and writes of the window placement.
//! - `review_queue_view_storage.rs`: reads and writes of the review queue's selection, scroll, tab and sidebar.
//! - `brief_storage.rs`: stored briefs (user data): one per review scope, its own row, pages and record lists; `brief_index` for the queue.
//! - `brief_widget_writing.rs`: writing a brief's widgets, each kind's parts in their own rows.
//! - `brief_widget_reading.rs`: reading a brief's pages and widgets back from their rows.
//! - `brief_anchor_columns.rs`: `AnchorColumns`, a code anchor as its four nullable columns.
//! - `stored_brief.rs`: `StoredBrief`, a checked studio with the record of its run.
//! - `brief_index.rs`: `BriefIndex`, which pull requests have a brief.
//! - `brief_sql_text.rs`: SQL text of the `qd-studio` enums a brief stores, and of the brief tables' own text columns (widget kind, anchor slot, scope list, code access).
//! - `scope_columns.rs`: a `ReviewScope` as its nullable columns, shared by notes and briefs.
//! - `database_connection.rs`: the database location and how a connection is opened.
//! - `schema_migrations.rs`: the append-only schema history and its runner.
//! - `pull_request_list_key.rs`: `PullRequestListKey`, the name of a cached list.
//! - `window_placement.rs`: `WindowPlacement`, the main window's saved bounds.
//! - `window_mode.rs`: `WindowMode`, windowed, maximized or fullscreen.
//! - `display_uuid.rs`: `DisplayUuid`, the display a window was on.
//! - `review_layout_state.rs`: `ReviewLayoutState`, the review pane's layout and filter toggles.
//! - `panel_visibility.rs`: `PanelVisibility`, shown or hidden.
//! - `panel_widths.rs`: `PanelWidths`, the dragged widths of the sidebar and side panel.
//! - `panel_widths_storage.rs`: reads and writes of the panel widths.
//! - `review_layout_storage.rs`: reads and writes of the review layout.
//! - `review_queue_view_state.rs`: `ReviewQueueViewState`, the review queue's saved selection and scroll.
//! - `detail_scroll_position.rs`: `DetailScrollPosition`, the detail pane's scroll as row and offset.
//! - `diff_view_settings.rs`: `DiffViewSettings`, how file diffs are shown.
//! - `diff_layout.rs`: `DiffLayout`, unified or side by side.
//! - `line_wrapping.rs`: `LineWrapping`, wrap or scroll long lines.
//! - `diff_option_sql_text.rs`: SQL text of the `qd-diff` enums in the settings.
//! - `diff_view_settings_storage.rs`: reads and writes of the diff view settings.
//! - `viewed_file.rs`: `ViewedFile`, a file marked as viewed.
//! - `patch_fingerprint.rs`: `PatchFingerprint`, a stable hash of a file's patch.
//! - `viewed_files_storage.rs`: reads and writes of viewed marks.
//! - `review_scope.rs`: `ReviewScope`, what viewed marks and review notes belong to: a pull request or a combined range of stack layers.
//! - `queue_tab.rs`: `QueueTab`, which list the queue shows.
//! - `watched_repositories_storage.rs`: reads and writes of the repositories the Others tab lists.
//! - `others_filter.rs`: `OthersFilter` and `RepositoryOthersFilter`, how the Others tab narrows a repository's pull requests.
//! - `updated_within.rs`: `UpdatedWithin`, how recently an Others pull request was updated.
//! - `others_filter_storage.rs`: reads and writes of the Others filters.
//! - `viewer_review_sql_text.rs`: the SQL text of `qd-github`'s `ViewerReview`.
//! - `hidden_branch_rule.rs`: `HiddenBranchRule`, a pattern hiding pull requests by branch.
//! - `branch_side.rs`: `BranchSide`, which branch of a pull request a rule looks at.
//! - `hidden_branch_rules_storage.rs`: reads and writes of the hidden branch rules.
//! - `folded_groups.rs`: `FoldedGroups` and `FoldedStack`, the sidebar groups the user folded.
//! - `folded_groups_storage.rs`: reads and writes of the folded groups.
//! - `stack_review_mode.rs`: `StackReviewMode`, one layer or combined layers.
//! - `cached_stack.rs`: `CachedStack`, a stack with the tag of its response.
//! - `stack_storage.rs`: reads and writes of cached stacks and their layers.
//! - `comparison_storage.rs`: reads and writes of the changed files between two commits.
//! - `file_text_storage.rs`: whole file contents by blob SHA (full or abbreviated).
//! - `review_note.rs`: `ReviewNote`, a local note or a GitHub draft comment.
//! - `note_id.rs`: `NoteId`, a note's id, made by the app.
//! - `note_destination.rs`: `NoteDestination`, local or GitHub.
//! - `note_anchor.rs`: `NoteAnchor`, a line (or lines) or a reply to a GitHub thread.
//! - `review_summary.rs`: `ReviewSummary`, the verdict and summary of a GitHub review.
//! - `review_notes_storage.rs`: reads and writes of notes and summaries.

mod appearance_settings_storage;
mod branch_side;
mod brief_anchor_columns;
mod brief_index;
mod brief_sql_text;
mod brief_storage;
mod brief_widget_reading;
mod brief_widget_writing;
mod cache_cleanup;
mod cache_settings_storage;
mod cache_ttl;
mod cache_usage;
mod cache_usage_storage;
mod cached_file_set;
mod cached_stack;
mod cached_statements;
mod cleanup_counts;
mod color_theme;
mod comparison_storage;
mod database_connection;
mod detail_scroll_position;
mod diff_layout;
mod diff_option_sql_text;
mod diff_view_settings;
mod diff_view_settings_storage;
mod display_uuid;
mod file_set_revision;
mod file_set_storage;
mod file_text_storage;
mod folded_groups;
mod folded_groups_storage;
mod github_cache;
mod hidden_branch_rule;
mod hidden_branch_rules_storage;
mod line_wrapping;
mod note_anchor;
mod note_destination;
mod note_id;
mod others_filter;
mod others_filter_storage;
mod panel_visibility;
mod panel_widths;
mod panel_widths_storage;
mod patch_fingerprint;
mod pull_request_list_key;
mod pull_request_storage;
mod queue_tab;
mod review_layout_state;
mod review_layout_storage;
mod review_note;
mod review_notes_storage;
mod review_queue_view_state;
mod review_queue_view_storage;
mod review_scope;
mod review_summary;
mod schema_migrations;
mod scope_columns;
mod sql_integers;
mod stack_review_mode;
mod stack_storage;
mod stored_brief;
mod ui_state_store;
mod unix_now;
mod updated_within;
mod viewed_file;
mod viewed_files_storage;
mod viewer_review_sql_text;
mod watched_repositories_storage;
mod window_mode;
mod window_placement;
mod window_placement_storage;

pub use branch_side::BranchSide;
pub use brief_index::BriefIndex;
pub use cache_ttl::CacheTtl;
pub use cache_usage::{CacheUsage, CachedData};
pub use cached_file_set::CachedFileSet;
pub use cached_stack::CachedStack;
pub use cleanup_counts::CleanupCounts;
pub use color_theme::ColorTheme;
pub use detail_scroll_position::DetailScrollPosition;
pub use diff_layout::DiffLayout;
pub use diff_view_settings::DiffViewSettings;
pub use display_uuid::DisplayUuid;
pub use file_set_revision::FileSetRevision;
pub use folded_groups::{FoldedGroups, FoldedStack};
pub use github_cache::GitHubCache;
pub use hidden_branch_rule::HiddenBranchRule;
pub use line_wrapping::LineWrapping;
pub use note_anchor::NoteAnchor;
pub use note_destination::NoteDestination;
pub use note_id::NoteId;
pub use others_filter::{OthersFilter, RepositoryOthersFilter};
pub use panel_visibility::PanelVisibility;
pub use panel_widths::PanelWidths;
pub use patch_fingerprint::PatchFingerprint;
pub use pull_request_list_key::PullRequestListKey;
pub use queue_tab::QueueTab;
pub use review_layout_state::ReviewLayoutState;
pub use review_note::ReviewNote;
pub use review_queue_view_state::ReviewQueueViewState;
pub use review_scope::ReviewScope;
pub use review_summary::ReviewSummary;
pub use stack_review_mode::StackReviewMode;
pub use stored_brief::StoredBrief;
pub use ui_state_store::UiStateStore;
pub use updated_within::UpdatedWithin;
pub use viewed_file::ViewedFile;
pub use window_mode::WindowMode;
pub use window_placement::WindowPlacement;
