use crate::PullRequestListKey;

/// Which list the review queue shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QueueTab {
    /// The viewer's own open pull requests.
    #[default]
    Authored,
    /// Open pull requests that request the viewer's review.
    ReviewRequested,
}

impl QueueTab {
    /// The cached list the tab shows.
    pub fn list_key(self) -> PullRequestListKey {
        match self {
            Self::Authored => PullRequestListKey::AUTHORED,
            Self::ReviewRequested => PullRequestListKey::REVIEW_REQUESTED,
        }
    }

    /// The value stored in SQLite; the column's CHECK lists the same.
    pub(crate) fn as_sql_text(self) -> &'static str {
        match self {
            Self::Authored => "authored",
            Self::ReviewRequested => "review_requested",
        }
    }

    /// Decodes a stored value; `None` for anything else.
    pub(crate) fn from_sql_text(text: &str) -> Option<Self> {
        match text {
            "authored" => Some(Self::Authored),
            "review_requested" => Some(Self::ReviewRequested),
            _ => None,
        }
    }
}
