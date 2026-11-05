//! Minimal, blocking GitHub REST client.
//!
//! Blocking on purpose: the app runs it on gpui's background executor, which
//! keeps a tokio runtime out of the process.
//!
//! Files:
//! - `github_client.rs`: `GitHubClient`, the authenticated REST client.
//! - `github_error.rs`: `GitHubError`, every way a call can fail.
//! - `resolve_token.rs`: finds the token from the environment or the `gh` CLI.
//! - `repo_ref.rs`: `RepoRef`, an owner/name pair.
//! - `user.rs`: `User`, a GitHub account as embedded in other objects.
//! - `pull_request.rs`: `PullRequest`, the normalized pull request.
//! - `pull_request_file.rs`: `PullRequestFile`, one changed file with its patch.
//! - `search_issue.rs`: the issue shape the search API returns for a pull request.
//! - `search_response.rs`: the envelope around search results.

mod github_client;
mod github_error;
mod pull_request;
mod pull_request_file;
mod repo_ref;
mod resolve_token;
mod search_issue;
mod search_response;
mod user;

pub use github_client::GitHubClient;
pub use github_error::GitHubError;
pub use pull_request::PullRequest;
pub use pull_request_file::PullRequestFile;
pub use repo_ref::RepoRef;
pub use resolve_token::resolve_token;
pub use user::User;
