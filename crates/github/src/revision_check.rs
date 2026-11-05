use crate::PullRequestRevision;

/// What one look at a pull request (`GET /pulls/{n}`) tells about its files:
/// the commits they depend on and how many there are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionCheck {
    pub revision: PullRequestRevision,
    /// GitHub's count of changed files, known before they are fetched.
    pub changed_files: u64,
}
