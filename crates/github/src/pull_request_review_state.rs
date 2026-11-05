use crate::ReviewThread;

/// What a review of one pull request starts from: its node id, who is
/// reviewing, the conversations on its diff, and the viewer's review still
/// pending on GitHub (begun in the browser), if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestReviewState {
    pub pull_request_node_id: String,
    pub viewer_login: String,
    /// The viewer opened it: GitHub refuses approving or requesting changes.
    pub viewer_is_author: bool,
    pub pending_review_id: Option<String>,
    pub threads: Vec<ReviewThread>,
}
