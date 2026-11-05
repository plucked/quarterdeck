use qc_github::ReviewEvent;

/// The verdict and summary of a GitHub review being written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewSummary {
    pub event: ReviewEvent,
    /// Markdown; may be empty.
    pub body: String,
}

/// The stored name of a review event; the column's CHECK lists the same.
pub(crate) fn event_sql_text(event: ReviewEvent) -> &'static str {
    match event {
        ReviewEvent::Comment => "comment",
        ReviewEvent::Approve => "approve",
        ReviewEvent::RequestChanges => "request_changes",
    }
}

/// Decodes a stored review event; `None` for anything else.
pub(crate) fn event_from_sql_text(text: &str) -> Option<ReviewEvent> {
    match text {
        "comment" => Some(ReviewEvent::Comment),
        "approve" => Some(ReviewEvent::Approve),
        "request_changes" => Some(ReviewEvent::RequestChanges),
        _ => None,
    }
}
