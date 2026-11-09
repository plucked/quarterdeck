/// Which version of a file a line number counts in: before the change (the
/// pull request's base) or after it (its head).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodeSide {
    Base,
    Head,
}

impl CodeSide {
    /// The word the agent writes for this side.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Head => "head",
        }
    }
}
