use qd_github::{ETag, PullRequestRevision};

/// What a cached file set was fetched for: the pull request's head and base
/// commits, and the tag of the pull request response that reported them,
/// so the next check can be a free conditional request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSetRevision {
    pub commits: PullRequestRevision,
    pub etag: Option<ETag>,
}
