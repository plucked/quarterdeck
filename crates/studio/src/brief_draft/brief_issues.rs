use super::BriefIssue;

const MAX_ISSUES: usize = 64;
const MAX_BYTES: usize = 64 * 1024;

/// Preserves the full count while bounding the concrete diagnostic response.
#[derive(Default)]
pub(crate) struct BriefIssues {
    pub(crate) issues: Vec<BriefIssue>,
    pub(crate) total: usize,
    bytes: usize,
}

impl BriefIssues {
    pub(crate) fn push(&mut self, issue: BriefIssue) {
        self.total += 1;
        let bytes = issue.storage_bytes();
        if self.issues.len() < MAX_ISSUES && self.bytes.saturating_add(bytes) <= MAX_BYTES {
            self.bytes += bytes;
            self.issues.push(issue);
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// Page identity is fixed by the addressed replacement, never inferred from a title.
    pub(crate) fn identify_page(&mut self, page_id: &str) {
        self.bytes = 0;
        self.issues.retain_mut(|issue| {
            issue.page_id = Some(page_id.into());
            let bytes = issue.storage_bytes();
            if self.bytes.saturating_add(bytes) > MAX_BYTES {
                return false;
            }
            self.bytes += bytes;
            true
        });
    }
}
