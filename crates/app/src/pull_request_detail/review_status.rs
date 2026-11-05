use gpui::SharedString;

/// How the last review action (a submission, an edit on GitHub, a copy) went.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ReviewStatus {
    #[default]
    Idle,
    /// Talking to GitHub; what about.
    Working(SharedString),
    /// It worked; the message says what happened.
    Done(SharedString),
    /// It failed; the message says why.
    Failed(SharedString),
}
