/// How a file changed at the git level, when that determines its `FileKind`
/// regardless of its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChange {
    /// Added, modified, or moved with edits: kind comes from the path.
    Content,
    /// Moved or copied with its content unchanged.
    UnchangedMove,
    /// Removed from the repository.
    Deleted,
}
