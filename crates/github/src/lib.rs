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
//! - `download_avatar.rs`: `download_avatar`, a user's avatar image from GitHub's avatar host.
//! - `other_pull_requests.rs`: `GitHubClient::other_pull_requests`, others' open pull requests in chosen repositories.
//! - `other_pull_requests_update.rs`: `OtherPullRequestsUpdate`, what one repository's sync found and where the next starts.
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
//! - `listed_pull_request_details.rs`: `ListedPullRequestDetails`, a listed pull request's branches and stack membership.
//! - `listed_details_query.rs`: the GraphQL query for those details and its response.
//! - `pull_request_branches.rs`: `PullRequestBranches`, the branches a pull request comes from and goes into.
//! - `pull_request_lookup_query.rs`: the GraphQL query for the branches and viewer review of chosen pull requests of one repository.
//! - `viewer_review.rs`: `ViewerReview`, the viewer's latest submitted review verdict.
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
//! - `check_outcome.rs`: `CheckOutcome`, where one check stands (failed, running, passed, …).
//! - `expected_checks.rs`: marks the checks the base branch requires and adds those not reported yet as expected.
//! - `status_check.rs`: `StatusCheck` and `CheckSource`, one check on a head commit and what produced it.
//! - `pull_request_checks.rs`: `PullRequestChecks` and `CheckDetails`, a head commit's checks or why they are hidden.
//! - `status_checks_query.rs`: the GraphQL query of a head commit's checks, page by page, and its answer.
//! - `status_check_requests.rs`: `GitHubClient::pull_request_checks`, every page of a pull request's checks.
//! - `merge_conflict.rs`: `MergeConflict` and `ConflictLocation`, a pull request (or a layer below it) that conflicts with its base.
//! - `layer_conflicts_query.rs`: the GraphQL query of whether chosen stack layers conflict with their bases.
//! - `workflow_jobs_response.rs`: the REST answers of a commit's Actions runs, their jobs and its statuses.
//! - `workflow_job_requests.rs`: `GitHubClient::workflow_job_checks`, a commit's checks read through GitHub Actions.

mod api_usage;
mod check_outcome;
mod conditional_response;
mod download_avatar;
mod download_progress;
mod etag;
mod expected_checks;
mod file_text;
mod file_text_location;
mod file_text_query;
mod file_text_requests;
mod github_client;
mod github_error;
mod graphql_response;
mod keychain_token;
mod label;
mod layer_conflicts_query;
mod line_side;
mod listed_details_query;
mod listed_pull_request_details;
mod merge_conflict;
mod merge_method;
mod merge_options;
mod merge_outcome;
mod merge_queries;
mod next_page_url;
mod other_pull_requests;
mod other_pull_requests_update;
mod pull_request;
mod pull_request_branches;
mod pull_request_checks;
mod pull_request_file;
mod pull_request_lookup_query;
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
mod status_check;
mod status_check_requests;
mod status_checks_query;
mod token_source;
mod unified_diff;
mod user;
mod viewer_request;
mod viewer_review;
mod workflow_job_requests;
mod workflow_jobs_response;

pub use api_usage::{ApiUsage, ApiUsageSnapshot};
pub use check_outcome::CheckOutcome;
pub use conditional_response::ConditionalResponse;
pub use download_avatar::download_avatar;
pub use download_progress::DownloadProgress;
pub use etag::ETag;
pub use file_text::FileText;
pub use file_text_location::FileTextLocation;
pub use github_client::{GitHubClient, MAX_LISTED_FILES};
pub use github_error::GitHubError;
pub use keychain_token::{delete_stored_token, store_token};
pub use label::Label;
pub use line_side::LineSide;
pub use listed_pull_request_details::ListedPullRequestDetails;
pub use merge_conflict::{ConflictLocation, MergeConflict};
pub use merge_method::MergeMethod;
pub use merge_options::{
    MergeOptions, MergePermissions, MergeQueueState, MergeReadiness, PullRequestState,
};
pub use merge_outcome::MergeOutcome;
pub use other_pull_requests_update::OtherPullRequestsUpdate;
pub use pull_request::PullRequest;
pub use pull_request_branches::PullRequestBranches;
pub use pull_request_checks::{CheckDetails, PullRequestChecks};
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
pub use status_check::{CheckSource, StatusCheck};
pub use token_source::TokenSource;
pub use user::User;
pub use viewer_review::ViewerReview;
