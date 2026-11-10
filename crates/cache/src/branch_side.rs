/// Which branch of a pull request a [`crate::HiddenBranchRule`] looks at.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BranchSide {
    /// The branch the changes come from.
    Head,
    /// The branch the changes go into.
    Base,
    /// Either of the two.
    #[default]
    Either,
}

impl BranchSide {
    /// Every side, in the order the settings offer them.
    pub const ALL: [Self; 3] = [Self::Either, Self::Head, Self::Base];

    /// The value stored in SQLite; the column's CHECK lists the same.
    pub(crate) fn as_sql_text(self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Base => "base",
            Self::Either => "either",
        }
    }

    /// Decodes a stored value; `None` for anything else.
    pub(crate) fn from_sql_text(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|side| side.as_sql_text() == text)
    }
}
