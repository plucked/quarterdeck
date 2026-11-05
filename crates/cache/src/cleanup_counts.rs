/// What a cleanup of expired cache entries removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CleanupCounts {
    pub file_sets: usize,
    pub pull_requests: usize,
    /// Combined diffs between two commits.
    pub comparisons: usize,
    pub stacks: usize,
}
