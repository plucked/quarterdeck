use qc_github::{PullRequest, PullRequestFile};

use crate::FileSetRevision;

/// A pull request's changed files as cached, in whatever version they were
/// fetched or last confirmed for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedFileSet {
    /// The pull request's `updated_at` when the files were fetched or last confirmed.
    pub updated_at: String,
    pub revision: FileSetRevision,
    pub files: Vec<PullRequestFile>,
}

impl CachedFileSet {
    /// Whether the files were confirmed for `pull_request` as it is now, so
    /// no request is needed. A newer `updated_at` (a comment, a review, a
    /// push) means the commits have to be checked.
    pub fn is_current_for(&self, pull_request: &PullRequest) -> bool {
        self.updated_at == pull_request.updated_at
    }
}
