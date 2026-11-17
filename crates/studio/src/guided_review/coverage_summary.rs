/// Coverage counts over the unique captured inventory, including unreadable files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageSummary {
    pub total_paths: usize,
    pub covered_paths: usize,
    pub missing_paths: usize,
}
