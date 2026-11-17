use crate::guided_review::DraftStatus;

use super::BriefIssue;

/// A replayable Brief command outcome; readiness still awaits host publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefDraftResponse {
    pub status: DraftStatus,
    pub revision: u64,
    pub issues: Vec<BriefIssue>,
    pub issue_count: usize,
    pub receipt: Option<String>,
}

impl BriefDraftResponse {
    pub fn is_error(&self) -> bool {
        self.status == DraftStatus::Rejected
    }

    pub(crate) fn storage_bytes(&self) -> usize {
        size_of::<Self>()
            + self
                .issues
                .iter()
                .map(BriefIssue::storage_bytes)
                .sum::<usize>()
            + self.receipt.as_ref().map_or(0, String::len)
    }
}
