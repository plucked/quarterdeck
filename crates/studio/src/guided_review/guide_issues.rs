use super::GuideIssue;

const MAX_ISSUES: usize = 64;
const MAX_ISSUE_BYTES: usize = 64 * 1024;

/// Counts every independent issue while retaining a deterministic bounded sample.
#[derive(Debug, Default)]
pub(crate) struct GuideIssues {
    pub(crate) issues: Vec<GuideIssue>,
    pub(crate) total: usize,
    encoded_bytes: usize,
}

impl GuideIssues {
    /// Oversized evidence is omitted whole, so retained paths remain exact.
    pub(crate) fn push(&mut self, issue: GuideIssue) {
        self.total = self.total.saturating_add(1);
        let bytes = issue.encoded_bytes();
        if self.issues.len() < MAX_ISSUES
            && self.encoded_bytes.saturating_add(bytes) <= MAX_ISSUE_BYTES
        {
            self.encoded_bytes += bytes;
            self.issues.push(issue);
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.total == 0
    }
}
