/// What submitting a review says about the pull request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReviewEvent {
    /// Comments only.
    #[default]
    Comment,
    Approve,
    RequestChanges,
}

impl ReviewEvent {
    /// GitHub's name for it (`PullRequestReviewEvent`).
    pub(crate) fn as_github(self) -> &'static str {
        match self {
            Self::Comment => "COMMENT",
            Self::Approve => "APPROVE",
            Self::RequestChanges => "REQUEST_CHANGES",
        }
    }
}
