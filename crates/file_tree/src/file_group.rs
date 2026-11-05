/// A group of changed files the filter can hide as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileGroup {
    /// Files marked viewed.
    Viewed,
    /// Test files and folders ([`crate::is_test_path`]).
    Tests,
    /// Generated files ([`crate::is_generated_path`]).
    Generated,
    /// Files moved or copied without changes to their content.
    UnchangedMoves,
}

impl FileGroup {
    /// Every group, in chip order.
    pub const ALL: [Self; 4] = [
        Self::Viewed,
        Self::Tests,
        Self::Generated,
        Self::UnchangedMoves,
    ];
}
