/// Whether long diff lines wrap or scroll sideways.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LineWrapping {
    Wrap,
    /// Lines stay on one row; the diff scrolls horizontally.
    #[default]
    Scroll,
}

impl LineWrapping {
    const ALL: [Self; 2] = [Self::Wrap, Self::Scroll];

    /// The value stored in SQLite; the table's CHECK constraint lists the same values.
    pub(crate) fn as_sql_text(self) -> &'static str {
        match self {
            Self::Wrap => "wrap",
            Self::Scroll => "scroll",
        }
    }

    /// Decodes a stored value; `None` for anything else.
    pub(crate) fn from_sql_text(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|wrapping| wrapping.as_sql_text() == text)
    }
}
