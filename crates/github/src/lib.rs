//! Minimal, blocking GitHub REST client.
//!
//! Blocking on purpose: the app runs it on gpui's background executor, which
//! keeps a tokio runtime out of the process.
//!
//! Files:
//! - `github_client.rs`: `GitHubClient`, the authenticated REST client.
//! - `github_error.rs`: `GitHubError`, every way a call can fail.
//! - `resolve_token.rs`: finds the token: the environment, the Keychain, then the `gh` CLI.
//! - `resolved_token.rs`: `ResolvedToken`, a token and where it was found.
//! - `token_source.rs`: `TokenSource`, the environment, the Keychain or the `gh` CLI.
//! - `keychain_token.rs`: the token saved in the login Keychain: read, save, remove.
//! - `viewer_request.rs`: `GitHubClient::viewer`, the account the token belongs to.
//! - `repo_ref.rs`: `RepoRef`, an owner/name pair.
//! - `user.rs`: `User`, a GitHub account as embedded in other objects.
//! - `label.rs`: `Label`, a pull request's label.
//! - `other_pull_requests.rs`: `GitHubClient::other_pull_requests`, others' open pull requests in chosen repositories.
//! - `repository_request.rs`: `GitHubClient::repository`, checks a repository exists.
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
//! - `graphql_response.rs`: the envelope of a GraphQL answer, data or errors.
//! - `line_side.rs`: `LineSide`, the old or new side of a diff line.
//! - `review_event.rs`: `ReviewEvent`, comment, approve or request changes.
//! - `review_thread.rs`: `ReviewThread` and `ThreadComment`, a conversation on the diff.
//! - `pull_request_review_state.rs`: `PullRequestReviewState`, what a review starts from.
//! - `review_submission.rs`: `ReviewSubmission`, a review to submit, and its parts.
//! - `review_queries.rs`: the GraphQL texts of reviewing and their answers.
//! - `review_requests.rs`: `GitHubClient`'s review calls: state, submit, edit, delete, resolve.
//! - `merge_method.rs`: `MergeMethod`, merge commit, squash or rebase.
//! - `merge_options.rs`: `MergeOptions`, what merging or closing can do now, with its parts.
//! - `merge_outcome.rs`: `MergeOutcome`, how an asynchronous merge ended or that it runs.
//! - `merge_queries.rs`: the merge options query and the asynchronous merge answers.
//! - `pull_request_requests.rs`: `GitHubClient`'s merge, close and branch deletion calls.
//! - `file_text.rs`: `FileText`, a file's whole content with its blob SHA.
//! - `file_text_location.rs`: `FileTextLocation`, a blob SHA or a path in a commit.
//! - `file_text_query.rs`: the GraphQL query of a file at a path, and its answer.
//! - `file_text_requests.rs`: `GitHubClient::file_text`, by blob (REST) or path (GraphQL).

mod api_usage;
mod conditional_response;
mod download_progress;
mod etag;
mod file_text;
mod file_text_location;
mod file_text_query;
mod file_text_requests;
mod github_client;
mod github_error;
mod graphql_response;
mod keychain_token;
mod label;
mod line_side;
mod listed_stack_membership;
mod merge_method;
mod merge_options;
mod merge_outcome;
mod merge_queries;
mod next_page_url;
mod other_pull_requests;
mod pull_request;
mod pull_request_file;
mod pull_request_requests;
mod pull_request_review_state;
mod pull_request_revision;
mod pull_request_stack;
mod rate_limit;
mod repo_ref;
mod repository_request;
mod resolve_token;
mod resolved_token;
mod review_event;
mod review_queries;
mod review_requests;
mod review_submission;
mod review_thread;
mod revision_check;
mod search_issue;
mod search_response;
mod stack_layer;
mod stack_membership;
mod stack_membership_query;
mod token_source;
mod unified_diff;
mod user;
mod viewer_request;

pub use api_usage::{ApiUsage, ApiUsageSnapshot};
pub use conditional_response::ConditionalResponse;
pub use download_progress::DownloadProgress;
pub use etag::ETag;
pub use file_text::FileText;
pub use file_text_location::FileTextLocation;
pub use github_client::{GitHubClient, MAX_LISTED_FILES};
pub use github_error::GitHubError;
pub use keychain_token::{delete_stored_token, store_token, stored_token};
pub use label::Label;
pub use line_side::LineSide;
pub use listed_stack_membership::ListedStackMembership;
pub use merge_method::MergeMethod;
pub use merge_options::{
    MergeOptions, MergePermissions, MergeQueueState, MergeReadiness, PullRequestState,
};
pub use merge_outcome::MergeOutcome;
pub use pull_request::PullRequest;
pub use pull_request_file::PullRequestFile;
pub use pull_request_review_state::PullRequestReviewState;
pub use pull_request_revision::PullRequestRevision;
pub use pull_request_stack::PullRequestStack;
pub use rate_limit::{RateLimit, RateLimitResource};
pub use repo_ref::RepoRef;
pub use resolve_token::resolve_token;
pub use resolved_token::ResolvedToken;
pub use review_event::ReviewEvent;
pub use review_submission::{NewReviewThread, NewThreadReply, ReviewSubmission, SubmittedPart};
pub use review_thread::{ReviewThread, ThreadComment, ThreadPermissions};
pub use revision_check::RevisionCheck;
pub use stack_layer::StackLayer;
pub use stack_membership::StackMembership;
pub use token_source::TokenSource;
pub use user::User;
