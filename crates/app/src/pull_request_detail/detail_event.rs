use qd_github::{RepoRef, StackMembership};

/// What the detail view tells the view that owns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailEvent {
    /// The pane scrolled; its position is worth saving.
    Scrolled,
    /// The user asked to show another layer of the shown stack.
    OpenLayer(LayerRequest),
    /// The pull request was merged or closed: the lists change.
    PullRequestChanged,
    /// The files shown are of another target (one layer or combined
    /// layers); the sidebar marks a stack under combined review.
    TargetChanged,
    /// A brief was written and stored; the sidebar marks pull requests
    /// that have one.
    BriefStored,
}

/// A stack layer to show: the pull request `number` of `repo`, with its
/// place in the stack (the issues API it may be fetched from lacks it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerRequest {
    pub repo: RepoRef,
    pub number: u64,
    pub membership: StackMembership,
}
