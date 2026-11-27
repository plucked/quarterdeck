use crate::PullRequestRevision;

/// What one look at a pull request (`GET /pulls/{n}`) tells about its files:
/// the commits they depend on, how many files changed, and how many lines were
/// added and deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionCheck {
    pub revision: PullRequestRevision,
    /// GitHub's count of changed files, known before they are fetched.
    pub changed_files: u64,
    /// GitHub's count of added lines across the pull request.
    pub additions: u64,
    /// GitHub's count of deleted lines across the pull request.
    pub deletions: u64,
}
