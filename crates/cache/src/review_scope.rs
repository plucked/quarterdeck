use qd_github::RepoRef;

/// What viewed marks and review notes belong to: one pull request, or a
/// combined review of a range of stack layers (a different diff, so its own
/// marks and notes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReviewScope {
    /// A pull request by its id (`PullRequest::id`).
    PullRequest(u64),
    /// Layers `first_number` up to `last_number` of a stack in `repo`.
    CombinedLayers {
        repo: RepoRef,
        first_number: u64,
        last_number: u64,
    },
}
