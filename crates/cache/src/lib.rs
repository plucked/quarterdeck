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

mod database_connection;
mod github_cache;
mod pull_request_list_key;
mod review_queue_view_state;
mod schema_migrations;
mod ui_state_store;
mod window_mode;
mod window_placement;

pub use github_cache::GitHubCache;
pub use pull_request_list_key::PullRequestListKey;
pub use review_queue_view_state::ReviewQueueViewState;
pub use ui_state_store::UiStateStore;
pub use window_mode::WindowMode;
pub use window_placement::WindowPlacement;
