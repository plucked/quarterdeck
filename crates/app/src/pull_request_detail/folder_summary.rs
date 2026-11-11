use super::folder_viewed_state::FolderViewedState;
use super::line_counts::LineCounts;

/// What a folder's tree row shows about the files under it, worked out once
/// per row refresh so drawing a row costs nothing: the matching files' changed
/// lines, count and heat, and whether the files the chips let through are viewed.
#[derive(Debug, Clone, Copy)]
pub struct FolderSummary {
    pub lines: LineCounts,
    /// Changed files in the subtree that pass the active filters, including moves and binaries.
    pub file_count: usize,
    /// Logarithmic position from cold (0) to warm (1), relative to the largest matching folder.
    pub heat: f64,
    pub viewed: FolderViewedState,
}
