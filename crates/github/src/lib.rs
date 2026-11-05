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
//! - `pull_request_revision.rs`: `PullRequestRevision`, the head and base commits of a pull request.
//! - `etag.rs`: `ETag`, the tag a conditional request sends back.
//! - `conditional_response.rs`: `ConditionalResponse`, not modified or a new value.
//! - `next_page_url.rs`: reads the next page from a `Link` header.
//! - `rate_limit.rs`: `RateLimit` and `RateLimitResource`, a budget GitHub reported.
//! - `api_usage.rs`: `ApiUsage`, requests sent and budgets left, shared by clones.
//! - `revision_check.rs`: `RevisionCheck`, a pull request's commits and file count.
//! - `download_progress.rs`: `DownloadProgress`, the bytes of a body received so far.
//! - `stack_membership.rs`: `StackMembership`, where a pull request sits in its stack.
//! - `listed_stack_membership.rs`: `ListedStackMembership`, a listed pull request's membership.
//! - `stack_membership_query.rs`: the GraphQL query for the memberships and its response.
//! - `pull_request_stack.rs`: `PullRequestStack`, a stack with its layers, and its response shape.
//! - `stack_layer.rs`: `StackLayer`, one pull request of a stack.
//! - `unified_diff.rs`: splits a `git diff` into changed files.

mod api_usage;
mod conditional_response;
mod download_progress;
mod etag;
mod github_client;
mod github_error;
mod listed_stack_membership;
mod next_page_url;
mod pull_request;
mod pull_request_file;
mod pull_request_revision;
mod pull_request_stack;
mod rate_limit;
mod repo_ref;
mod resolve_token;
mod revision_check;
mod search_issue;
mod search_response;
mod stack_layer;
mod stack_membership;
mod stack_membership_query;
mod unified_diff;
mod user;

pub use api_usage::{ApiUsage, ApiUsageSnapshot};
pub use conditional_response::ConditionalResponse;
pub use download_progress::DownloadProgress;
pub use etag::ETag;
pub use github_client::GitHubClient;
pub use github_error::GitHubError;
pub use listed_stack_membership::ListedStackMembership;
pub use pull_request::PullRequest;
pub use pull_request_file::PullRequestFile;
pub use pull_request_revision::PullRequestRevision;
pub use pull_request_stack::PullRequestStack;
pub use rate_limit::{RateLimit, RateLimitResource};
pub use repo_ref::RepoRef;
pub use resolve_token::resolve_token;
pub use revision_check::RevisionCheck;
pub use stack_layer::StackLayer;
pub use stack_membership::StackMembership;
pub use user::User;
