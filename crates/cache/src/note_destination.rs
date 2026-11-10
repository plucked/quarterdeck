/// Where a review note goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoteDestination {
    /// Stays on this machine, to copy for an agent.
    Local,
    /// A draft of a GitHub review comment, posted when the review is submitted.
    GitHub,
}

impl NoteDestination {
    const ALL: [Self; 2] = [Self::Local, Self::GitHub];

    /// The value stored in SQLite; the column's CHECK lists the same.
    pub(crate) fn as_sql_text(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::GitHub => "github",
        }
    }

    /// Decodes a stored value; `None` for anything else.
    pub(crate) fn from_sql_text(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|destination| destination.as_sql_text() == text)
    }
}
