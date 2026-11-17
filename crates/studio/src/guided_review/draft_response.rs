use super::{CoverageSummary, DraftStatus, GuideIssue, ReadinessReceipt};

/// A replayable command outcome. A readiness receipt does not claim publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftResponse {
    pub status: DraftStatus,
    pub revision: u64,
    pub coverage: CoverageSummary,
    pub issues: Vec<GuideIssue>,
    pub issue_count: usize,
    pub receipt: Option<ReadinessReceipt>,
}

impl DraftResponse {
    pub fn is_error(&self) -> bool {
        self.status == DraftStatus::Rejected
    }

    /// Response memory is charged to the run's bounded replay ledger.
    pub(crate) fn storage_bytes(&self) -> usize {
        size_of::<Self>()
            + self
                .issues
                .iter()
                .map(GuideIssue::encoded_bytes)
                .sum::<usize>()
            + self
                .receipt
                .as_ref()
                .map_or(0, |receipt| receipt.as_str().len())
    }
}
