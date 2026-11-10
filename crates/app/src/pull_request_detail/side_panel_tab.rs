/// What the side panel shows: the files, the review, the pull request's
/// actions, or its CI checks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SidePanelTab {
    /// The file tree with its filter.
    #[default]
    Files,
    /// The review: the user's comments, and submitting or copying them.
    Review,
    /// Merging or closing the pull request.
    Actions,
    /// The checks on the shown pull request's head commit.
    Checks,
}
