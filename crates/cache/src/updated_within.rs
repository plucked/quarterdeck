/// How recently a pull request must have been updated to show in the
/// Others tab.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UpdatedWithin {
    #[default]
    AnyTime,
    Week,
    Month,
}

impl UpdatedWithin {
    pub const ALL: [Self; 3] = [Self::AnyTime, Self::Week, Self::Month];

    /// The limit in seconds; `None` for any time.
    pub fn seconds(self) -> Option<i64> {
        match self {
            Self::AnyTime => None,
            Self::Week => Some(7 * 24 * 60 * 60),
            Self::Month => Some(30 * 24 * 60 * 60),
        }
    }

    /// The value stored in SQLite; the column's CHECK lists the same.
    pub(crate) fn as_sql_text(self) -> &'static str {
        match self {
            Self::AnyTime => "any",
            Self::Week => "week",
            Self::Month => "month",
        }
    }

    /// Decodes a stored value; `None` for anything else.
    pub(crate) fn from_sql_text(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|within| within.as_sql_text() == text)
    }
}
