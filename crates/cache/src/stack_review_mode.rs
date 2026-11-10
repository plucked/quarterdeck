/// How a stacked pull request is reviewed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StackReviewMode {
    /// One layer's own changes, as its pull request shows them.
    #[default]
    Layer,
    /// The changes of several layers as one diff, from the base of the first
    /// to the head of the last.
    Combined,
}

impl StackReviewMode {
    const ALL: [Self; 2] = [Self::Layer, Self::Combined];

    /// The other mode, for a toggle.
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            Self::Layer => Self::Combined,
            Self::Combined => Self::Layer,
        }
    }

    /// The value stored in SQLite; the column's CHECK lists the same.
    pub(crate) fn as_sql_text(self) -> &'static str {
        match self {
            Self::Layer => "layer",
            Self::Combined => "combined",
        }
    }

    /// Decodes a stored value; `None` for anything else.
    pub(crate) fn from_sql_text(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.as_sql_text() == text)
    }
}
