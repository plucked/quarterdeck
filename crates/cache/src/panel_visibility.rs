/// Whether a collapsible panel (the file tree) is shown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PanelVisibility {
    #[default]
    Shown,
    Hidden,
}

impl PanelVisibility {
    /// The other state, for a toggle.
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            Self::Shown => Self::Hidden,
            Self::Hidden => Self::Shown,
        }
    }

    /// The value stored in SQLite (`1` shown, `0` hidden); the column's CHECK lists the same.
    pub(crate) fn as_sql_integer(self) -> i64 {
        match self {
            Self::Shown => 1,
            Self::Hidden => 0,
        }
    }

    /// Decodes a stored value; `None` for anything else.
    pub(crate) fn from_sql_integer(value: i64) -> Option<Self> {
        match value {
            1 => Some(Self::Shown),
            0 => Some(Self::Hidden),
            _ => None,
        }
    }
}
