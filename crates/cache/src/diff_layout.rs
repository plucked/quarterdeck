/// Whether a file diff shows old and new lines in one column or side by side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DiffLayout {
    /// One column: removed lines above the added lines replacing them.
    #[default]
    Unified,
    /// Old lines on the left, new lines on the right.
    Split,
}

impl DiffLayout {
    const ALL: [Self; 2] = [Self::Unified, Self::Split];

    /// The value stored in SQLite; the table's CHECK constraint lists the same values.
    pub(crate) fn as_sql_text(self) -> &'static str {
        match self {
            Self::Unified => "unified",
            Self::Split => "split",
        }
    }

    /// Decodes a stored value; `None` for anything else.
    pub(crate) fn from_sql_text(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|layout| layout.as_sql_text() == text)
    }
}
