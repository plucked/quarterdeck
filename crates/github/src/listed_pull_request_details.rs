use crate::{PullRequestBranches, RepoRef, StackMembership, ViewerReview};

/// What the listed details query reports of one pull request in the
/// viewer's lists: its branches, the viewer's latest review and where it
/// sits in a stack; matched to
/// listed pull requests by repository and number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedPullRequestDetails {
    pub repo: RepoRef,
    pub number: u64,
    pub branches: PullRequestBranches,
    pub viewer_review: Option<ViewerReview>,
    /// `None` for a standalone pull request.
    pub stack: Option<StackMembership>,
}
