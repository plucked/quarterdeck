use qd_github::ReviewEvent;

/// The verdict and summary of a GitHub review being written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewSummary {
    pub event: ReviewEvent,
    /// Markdown; may be empty.
    pub body: String,
}
