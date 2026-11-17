use super::{BucketSummary, CoverageSummary};

/// Current accepted progress, kept separate from any published guided review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftView {
    pub run_id: String,
    pub snapshot_id: String,
    pub revision: u64,
    pub sealed: bool,
    pub buckets: Vec<BucketSummary>,
    pub coverage: CoverageSummary,
}
