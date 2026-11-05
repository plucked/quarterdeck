/// How one file shows, as decided by the view that owns the file list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FileDiffState {
    /// Only the header shows.
    pub is_collapsed: bool,
    /// The user marked the file as viewed at its current patch.
    pub is_viewed: bool,
    /// Big enough to start collapsed; a collapsed large file offers to expand.
    pub is_large: bool,
}
