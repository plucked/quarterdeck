/// Which side of a diff a line is on: the old file (removed lines) or the
/// new one (added and unchanged lines). GitHub calls them `LEFT` and `RIGHT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineSide {
    Old,
    New,
}

impl LineSide {
    /// GitHub's name for the side (`DiffSide`).
    pub(crate) fn as_github(self) -> &'static str {
        match self {
            Self::Old => "LEFT",
            Self::New => "RIGHT",
        }
    }

    /// Decodes GitHub's name; anything but `LEFT` is the new side.
    pub(crate) fn from_github(value: &str) -> Self {
        match value {
            "LEFT" => Self::Old,
            _ => Self::New,
        }
    }
}
