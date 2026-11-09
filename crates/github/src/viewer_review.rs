/// The viewer's latest submitted review of a pull request, from GraphQL
/// (`PullRequest.viewerLatestReview.state`). A pending (unsubmitted) review
/// counts as none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerReview {
    Approved,
    ChangesRequested,
    Commented,
    /// A review that was dismissed afterwards.
    Dismissed,
}

impl ViewerReview {
    /// Reads GraphQL's `PullRequestReviewState`; `None` for `PENDING` and
    /// for states GitHub may add later.
    pub(crate) fn from_graphql_state(state: &str) -> Option<Self> {
        match state {
            "APPROVED" => Some(Self::Approved),
            "CHANGES_REQUESTED" => Some(Self::ChangesRequested),
            "COMMENTED" => Some(Self::Commented),
            "DISMISSED" => Some(Self::Dismissed),
            _ => None,
        }
    }
}
