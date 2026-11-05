//! The app's local SQLite database: cached GitHub data, so the app shows the
//! last known state instantly and works offline, and the UI state it
//! restores after a relaunch. All state is stored in typed columns.
//!
//! Files:
//! - `github_cache.rs`: `GitHubCache`, cached pull requests and lists.
//! - `file_set_storage.rs`: a pull request's cached changed files, and confirming them.
//! - `cache_cleanup.rs`: removing expired cache entries, and clearing the cache.
//! - `cached_file_set.rs`: `CachedFileSet`, cached files with what they were fetched for.
//! - `file_set_revision.rs`: `FileSetRevision`, the commits and response tag of a file set.
//! - `cleanup_counts.rs`: `CleanupCounts`, what a cleanup removed.
//! - `cache_ttl.rs`: `CacheTtl`, how long unused cached data is kept.
//! - `cache_settings_storage.rs`: reads and writes of the cache TTL.
//! - `sql_integers.rs`: checked conversions between GitHub numbers and SQLite integers.
//! - `unix_now.rs`: the current time as stored.
//! - `ui_state_store.rs`: `UiStateStore`, reads and writes of the UI state.
//! - `database_connection.rs`: the database location and how a connection is opened.
//! - `schema_migrations.rs`: the append-only schema history and its runner.
//! - `pull_request_list_key.rs`: `PullRequestListKey`, the name of a cached list.
//! - `window_placement.rs`: `WindowPlacement`, the main window's saved bounds.
//! - `window_mode.rs`: `WindowMode`, windowed, maximized or fullscreen.
//! - `display_uuid.rs`: `DisplayUuid`, the display a window was on.
//! - `review_layout_state.rs`: `ReviewLayoutState`, the review pane's layout and filter toggles.
//! - `panel_visibility.rs`: `PanelVisibility`, shown or hidden.
//! - `review_layout_storage.rs`: reads and writes of the review layout.
//! - `review_queue_view_state.rs`: `ReviewQueueViewState`, the review queue's saved selection and scroll.
//! - `detail_scroll_position.rs`: `DetailScrollPosition`, the detail pane's scroll as row and offset.
//! - `diff_view_settings.rs`: `DiffViewSettings`, how file diffs are shown.
//! - `diff_layout.rs`: `DiffLayout`, unified or side by side.
//! - `line_wrapping.rs`: `LineWrapping`, wrap or scroll long lines.
//! - `diff_option_sql_text.rs`: SQL text of the `qc-diff` enums in the settings.
//! - `diff_view_settings_storage.rs`: reads and writes of the diff view settings.
//! - `viewed_file.rs`: `ViewedFile`, a file marked as viewed.
//! - `patch_fingerprint.rs`: `PatchFingerprint`, a stable hash of a file's patch.
//! - `viewed_files_storage.rs`: reads and writes of viewed marks.
//! - `viewed_scope.rs`: `ViewedScope`, a pull request or a combined range of stack layers.
//! - `queue_tab.rs`: `QueueTab`, which list the queue shows.
//! - `stack_review_mode.rs`: `StackReviewMode`, one layer or combined layers.
//! - `cached_stack.rs`: `CachedStack`, a stack with the tag of its response.
//! - `stack_storage.rs`: reads and writes of cached stacks and their layers.
//! - `comparison_storage.rs`: reads and writes of the changed files between two commits.

mod cache_cleanup;
mod cache_settings_storage;
mod cache_ttl;
mod cached_file_set;
mod cached_stack;
mod cleanup_counts;
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
mod github_cache;
mod line_wrapping;
mod panel_visibility;
mod patch_fingerprint;
mod pull_request_list_key;
mod queue_tab;
mod review_layout_state;
mod review_layout_storage;
mod review_queue_view_state;
mod schema_migrations;
mod sql_integers;
mod stack_review_mode;
mod stack_storage;
mod ui_state_store;
mod unix_now;
mod viewed_file;
mod viewed_files_storage;
mod viewed_scope;
mod window_mode;
mod window_placement;

pub use cache_ttl::CacheTtl;
pub use cached_file_set::CachedFileSet;
pub use cached_stack::CachedStack;
pub use cleanup_counts::CleanupCounts;
pub use detail_scroll_position::DetailScrollPosition;
pub use diff_layout::DiffLayout;
pub use diff_view_settings::DiffViewSettings;
pub use display_uuid::DisplayUuid;
pub use file_set_revision::FileSetRevision;
pub use github_cache::GitHubCache;
pub use line_wrapping::LineWrapping;
pub use panel_visibility::PanelVisibility;
pub use patch_fingerprint::PatchFingerprint;
pub use pull_request_list_key::PullRequestListKey;
pub use queue_tab::QueueTab;
pub use review_layout_state::ReviewLayoutState;
pub use review_queue_view_state::ReviewQueueViewState;
pub use stack_review_mode::StackReviewMode;
pub use ui_state_store::UiStateStore;
pub use viewed_file::ViewedFile;
pub use viewed_scope::ViewedScope;
pub use window_mode::WindowMode;
pub use window_placement::WindowPlacement;
