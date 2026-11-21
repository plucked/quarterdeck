use super::layer_request::LayerRequest;

/// What the detail view tells the view that owns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailEvent {
    /// The pane scrolled; its position is worth saving.
    Scrolled,
    /// The user asked to show another layer of the shown stack.
    OpenLayer(LayerRequest),
    /// Navigate within the branch-derived queue tree.
    OpenQueueLayer {
        repo: qd_github::RepoRef,
        number: u64,
    },
    /// A merge, close or submitted review may change queue membership.
    PullRequestChanged(u64),
    /// The files shown are of another target (one layer or combined
    /// layers); the sidebar marks a stack under combined review.
    TargetChanged,
    /// A brief was written and stored; the sidebar marks pull requests
    /// that have one.
    BriefStored,
}
