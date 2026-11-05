/// What the filter looks at in one changed file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilteredFile<'path> {
    pub path: &'path str,
    pub is_viewed: bool,
    /// Moved or copied with its content unchanged.
    pub is_unchanged_move: bool,
}
