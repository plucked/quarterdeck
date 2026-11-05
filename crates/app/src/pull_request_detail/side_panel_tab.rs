/// What the side panel shows: the files or the review.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SidePanelTab {
    /// The file tree with its filter.
    #[default]
    Files,
    /// The review: the user's comments, and submitting or copying them.
    Review,
}
