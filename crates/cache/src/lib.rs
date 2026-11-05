//! The app's local SQLite database: cached GitHub data, so the app shows the
//! last known state instantly and works offline, and the UI state it
//! restores after a relaunch. All state is stored in typed columns.
//!
//! Files:
//! - `github_cache.rs`: `GitHubCache`, reads and writes of cached GitHub data.
//! - `ui_state_store.rs`: `UiStateStore`, reads and writes of the UI state.
//! - `database_connection.rs`: the database location and how a connection is opened.
//! - `schema_migrations.rs`: the append-only schema history and its runner.
//! - `pull_request_list_key.rs`: `PullRequestListKey`, the name of a cached list.
//! - `window_placement.rs`: `WindowPlacement`, the main window's saved bounds.
//! - `window_mode.rs`: `WindowMode`, windowed, maximized or fullscreen.
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

mod database_connection;
mod detail_scroll_position;
mod diff_layout;
mod diff_option_sql_text;
mod diff_view_settings;
mod diff_view_settings_storage;
mod github_cache;
mod line_wrapping;
mod patch_fingerprint;
mod pull_request_list_key;
mod review_queue_view_state;
mod schema_migrations;
mod ui_state_store;
mod viewed_file;
mod viewed_files_storage;
mod window_mode;
mod window_placement;

pub use detail_scroll_position::DetailScrollPosition;
pub use diff_layout::DiffLayout;
pub use diff_view_settings::DiffViewSettings;
pub use github_cache::GitHubCache;
pub use line_wrapping::LineWrapping;
pub use patch_fingerprint::PatchFingerprint;
pub use pull_request_list_key::PullRequestListKey;
pub use review_queue_view_state::ReviewQueueViewState;
pub use ui_state_store::UiStateStore;
pub use viewed_file::ViewedFile;
pub use window_mode::WindowMode;
pub use window_placement::WindowPlacement;
