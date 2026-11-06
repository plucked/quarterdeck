/// Totals over a pull request's changed files, for the summary bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiffSummary {
    pub file_count: usize,
    pub additions: u64,
    pub deletions: u64,
}
