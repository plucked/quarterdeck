//! Local SQLite cache for GitHub data, so the app shows the last known state
//! instantly and works offline.
//!
//! Files:
//! - `github_cache.rs`: `GitHubCache`, the connection and every read and write.
//! - `schema_migrations.rs`: the append-only schema history and its runner.
//! - `pull_request_list_key.rs`: `PullRequestListKey`, the name of a cached list.

mod github_cache;
mod pull_request_list_key;
mod schema_migrations;

pub use github_cache::GitHubCache;
pub use pull_request_list_key::PullRequestListKey;
