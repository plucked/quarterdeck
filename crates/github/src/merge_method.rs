/// How a pull request's commits land on its base branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMethod {
    /// A merge commit.
    Merge,
    /// One commit with every change.
    Squash,
    /// The commits replayed onto the base.
    Rebase,
}

impl MergeMethod {
    /// The REST name (`merge_method`).
    pub(crate) fn as_rest(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Rebase => "rebase",
        }
    }

    /// The button text.
    pub fn label(self) -> &'static str {
        match self {
            Self::Merge => "Merge commit",
            Self::Squash => "Squash and merge",
            Self::Rebase => "Rebase and merge",
        }
    }
}
