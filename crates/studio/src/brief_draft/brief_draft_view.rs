use super::BriefPageSummary;

/// Accepted private progress, separate from the currently published Brief.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefDraftView {
    pub run_id: String,
    pub snapshot_id: String,
    pub revision: u64,
    pub sealed: bool,
    pub lead: String,
    pub pages: Vec<BriefPageSummary>,
}
