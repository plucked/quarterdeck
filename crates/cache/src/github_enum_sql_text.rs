//! SQL text for the `qd-github` enums the cache stores. They are mapped
//! here, beside their stores, because `qd-github` knows nothing of SQLite;
//! each column's CHECK lists the same values.

use qd_github::{LineSide, ReviewEvent, ViewerReview};

const VIEWER_REVIEWS: [ViewerReview; 4] = [
    ViewerReview::Approved,
    ViewerReview::ChangesRequested,
    ViewerReview::Commented,
    ViewerReview::Dismissed,
];
const REVIEW_EVENTS: [ReviewEvent; 3] = [
    ReviewEvent::Comment,
    ReviewEvent::Approve,
    ReviewEvent::RequestChanges,
];
const LINE_SIDES: [LineSide; 2] = [LineSide::Old, LineSide::New];

/// The value stored in `pull_requests.viewer_review` for `review`.
pub(crate) fn viewer_review_as_sql_text(review: ViewerReview) -> &'static str {
    match review {
        ViewerReview::Approved => "approved",
        ViewerReview::ChangesRequested => "changes_requested",
        ViewerReview::Commented => "commented",
        ViewerReview::Dismissed => "dismissed",
    }
}

/// Decodes a stored viewer review; `None` for anything else.
pub(crate) fn viewer_review_from_sql_text(text: &str) -> Option<ViewerReview> {
    VIEWER_REVIEWS
        .into_iter()
        .find(|review| viewer_review_as_sql_text(*review) == text)
}

/// The value stored in `review_summaries.event` for `event`.
pub(crate) fn review_event_as_sql_text(event: ReviewEvent) -> &'static str {
    match event {
        ReviewEvent::Comment => "comment",
        ReviewEvent::Approve => "approve",
        ReviewEvent::RequestChanges => "request_changes",
    }
}

/// Decodes a stored review event; `None` for anything else.
pub(crate) fn review_event_from_sql_text(text: &str) -> Option<ReviewEvent> {
    REVIEW_EVENTS
        .into_iter()
        .find(|event| review_event_as_sql_text(*event) == text)
}

/// The value stored in `review_notes.side` for `side`.
pub(crate) fn line_side_as_sql_text(side: LineSide) -> &'static str {
    match side {
        LineSide::Old => "old",
        LineSide::New => "new",
    }
}

/// Decodes a stored line side; `None` for anything else.
pub(crate) fn line_side_from_sql_text(text: &str) -> Option<LineSide> {
    LINE_SIDES
        .into_iter()
        .find(|side| line_side_as_sql_text(*side) == text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every viewer review, review event and line side decodes from the
    /// text it is stored as.
    #[test]
    fn github_enums_round_trip_through_sql_text() {
        for review in VIEWER_REVIEWS {
            assert_eq!(
                viewer_review_from_sql_text(viewer_review_as_sql_text(review)),
                Some(review)
            );
        }
        for event in REVIEW_EVENTS {
            assert_eq!(
                review_event_from_sql_text(review_event_as_sql_text(event)),
                Some(event)
            );
        }
        for side in LINE_SIDES {
            assert_eq!(
                line_side_from_sql_text(line_side_as_sql_text(side)),
                Some(side)
            );
        }
    }
}
