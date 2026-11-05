/// Lines a file adds and removes, or the files under a folder together.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineCounts {
    pub additions: u64,
    pub deletions: u64,
}

impl LineCounts {
    /// Both counts summed.
    #[must_use]
    pub fn plus(self, other: Self) -> Self {
        Self {
            additions: self.additions + other.additions,
            deletions: self.deletions + other.deletions,
        }
    }

    /// The larger of each count, for sizing columns that must fit both.
    #[must_use]
    pub fn widest(self, other: Self) -> Self {
        Self {
            additions: self.additions.max(other.additions),
            deletions: self.deletions.max(other.deletions),
        }
    }
}
