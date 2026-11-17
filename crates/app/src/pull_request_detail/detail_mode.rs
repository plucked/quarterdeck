/// What the detail pane's main column shows under the header.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DetailMode {
    /// The pull request's Markdown description.
    Summary,
    /// The changed files' diffs.
    #[default]
    Diff,
    /// The saved walkthrough, with one step's guidance and ordered file diffs.
    GuidedReview,
    /// The AI-written brief of the change.
    Brief,
}

impl DetailMode {
    /// Both review surfaces use the selected layer or combined comparison.
    pub(super) fn shows_diff(self) -> bool {
        matches!(self, Self::Diff | Self::GuidedReview)
    }
}
