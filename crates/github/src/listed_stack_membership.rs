use crate::{RepoRef, StackMembership};

/// The stack membership of one pull request in the viewer's lists, as the
/// stack membership query reports it; matched to listed pull requests by
/// repository and number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedStackMembership {
    pub repo: RepoRef,
    pub number: u64,
    pub membership: StackMembership,
}
