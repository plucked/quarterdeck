use super::review_target::ReviewTarget;

/// Which brief: the one of pull request `pull_request_id` at `target`, so a
/// layer and its combined range each have their own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefKey {
    pub pull_request_id: u64,
    pub target: ReviewTarget,
}
