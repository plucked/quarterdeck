use crate::PullRequestListKey;

/// Which list the review queue shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QueueTab {
    /// The viewer's own open pull requests.
    #[default]
    Authored,
    /// Open pull requests that request the viewer's review.
    ReviewRequested,
    /// Others' open pull requests in the watched repositories.
    Others,
}

impl QueueTab {
    const ALL: [Self; 3] = [Self::Authored, Self::ReviewRequested, Self::Others];

    /// The cached list the tab shows.
    pub fn list_key(self) -> PullRequestListKey {
        match self {
            Self::Authored => PullRequestListKey::AUTHORED,
            Self::ReviewRequested => PullRequestListKey::REVIEW_REQUESTED,
            Self::Others => PullRequestListKey::OTHERS,
        }
    }

    /// The value stored in SQLite; the column's CHECK lists the same.
    pub(crate) fn as_sql_text(self) -> &'static str {
        match self {
            Self::Authored => "authored",
            Self::ReviewRequested => "review_requested",
            Self::Others => "others",
        }
    }

    /// Decodes a stored value; `None` for anything else.
    pub(crate) fn from_sql_text(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.as_sql_text() == text)
    }
}
