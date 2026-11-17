use super::GuideIssueCode;

/// Concrete evidence for one repair, with pointers into the complete bucket input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuideIssue {
    pub code: GuideIssueCode,
    pub bucket_id: Option<String>,
    pub path: Option<String>,
    pub item_indexes: Vec<usize>,
    pub locations: Vec<String>,
    pub selectors: Vec<String>,
    pub message: String,
}

impl GuideIssue {
    /// Begins a diagnostic without guessing an item or path that is not involved.
    pub(crate) fn new(code: GuideIssueCode, message: impl Into<String>) -> Self {
        Self {
            code,
            bucket_id: None,
            path: None,
            item_indexes: Vec::new(),
            locations: Vec::new(),
            selectors: Vec::new(),
            message: message.into(),
        }
    }

    /// Conservatively bounds encoded diagnostic size, including JSON escaping.
    pub(crate) fn encoded_bytes(&self) -> usize {
        let text_bytes = self.bucket_id.as_ref().map_or(0, String::len)
            + self.path.as_ref().map_or(0, String::len)
            + self.locations.iter().map(String::len).sum::<usize>()
            + self.selectors.iter().map(String::len).sum::<usize>()
            + self.message.len();
        text_bytes.saturating_mul(6).saturating_add(512)
    }
}
