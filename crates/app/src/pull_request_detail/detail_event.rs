use qc_github::{RepoRef, StackMembership};

/// What the detail view tells the view that owns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailEvent {
    /// The pane scrolled; its position is worth saving.
    Scrolled,
    /// The user asked to show another layer of the shown stack.
    OpenLayer(LayerRequest),
}

/// A stack layer to show: the pull request `number` of `repo`, with its
/// place in the stack (the issues API it may be fetched from lacks it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerRequest {
    pub repo: RepoRef,
    pub number: u64,
    pub membership: StackMembership,
}
