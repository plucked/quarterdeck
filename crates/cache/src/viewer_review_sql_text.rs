//! The SQL text of `qd-github`'s [`ViewerReview`], stored in
//! `pull_requests.viewer_review`; the column's CHECK lists the same values.

use qd_github::ViewerReview;

/// The value stored for `review`.
pub(crate) fn viewer_review_as_sql_text(review: ViewerReview) -> &'static str {
    match review {
        ViewerReview::Approved => "approved",
        ViewerReview::ChangesRequested => "changes_requested",
        ViewerReview::Commented => "commented",
        ViewerReview::Dismissed => "dismissed",
    }
}

/// Decodes a stored value; `None` for anything else.
pub(crate) fn viewer_review_from_sql_text(text: &str) -> Option<ViewerReview> {
    match text {
        "approved" => Some(ViewerReview::Approved),
        "changes_requested" => Some(ViewerReview::ChangesRequested),
        "commented" => Some(ViewerReview::Commented),
        "dismissed" => Some(ViewerReview::Dismissed),
        _ => None,
    }
}
