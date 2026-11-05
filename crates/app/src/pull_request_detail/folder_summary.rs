use super::folder_viewed_state::FolderViewedState;
use super::line_counts::LineCounts;

/// What a folder's tree row shows about the files under it, worked out once
/// per row refresh so drawing a row costs nothing: their changed lines, and
/// how many of those the chips let through are viewed.
#[derive(Debug, Clone, Copy)]
pub struct FolderSummary {
    pub lines: LineCounts,
    pub viewed: FolderViewedState,
}
