use crate::PullRequest;

/// What [`crate::GitHubClient::other_pull_requests`] found for one
/// repository: its open pull requests by others, and where the next sync
/// starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtherPullRequestsUpdate {
    /// Most recently updated first.
    pub pull_requests: Vec<PullRequest>,
    /// GitHub's `updated_at` of the newest item read (issues and closed
    /// pull requests count): the next sync asks only for changes since.
    /// `None` when the repository had nothing to read.
    pub synced_until: Option<String>,
}
