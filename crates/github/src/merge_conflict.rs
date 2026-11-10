/// A pull request that conflicts with the branch it targets: the one whose
/// checks are shown, or a layer below it in its stack. GitHub starts no
/// `pull_request` workflow runs while it cannot build the merge commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeConflict {
    pub number: u64,
    /// The branch it conflicts with.
    pub base_ref: String,
    pub location: ConflictLocation,
}

/// Where the conflicting pull request sits relative to the shown one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictLocation {
    /// The shown pull request itself.
    Shown,
    /// A layer below it in its stack.
    LayerBelow,
}
