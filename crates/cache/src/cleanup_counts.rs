/// What a cleanup of expired cache entries removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CleanupCounts {
    pub file_sets: usize,
    pub pull_requests: usize,
}
