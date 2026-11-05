use qd_github::{ETag, PullRequestStack};

/// A stack as last fetched, with the tag of that response for a free
/// conditional check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedStack {
    pub stack: PullRequestStack,
    pub etag: Option<ETag>,
}
