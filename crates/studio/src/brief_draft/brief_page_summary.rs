/// Accepted page metadata for recovering the draft's stable page identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefPageSummary {
    pub page_id: String,
    pub title: String,
    pub widget_count: usize,
}
