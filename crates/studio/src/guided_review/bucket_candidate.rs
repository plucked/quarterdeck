use super::ItemCandidate;

/// A complete bucket proposal. Its stable ID and display order travel separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketCandidate {
    pub label: String,
    pub description: String,
    pub suggest_viewed: bool,
    pub items: Vec<ItemCandidate>,
}

impl BucketCandidate {
    /// Accounts for owned input text and collection slots before validation.
    pub(crate) fn storage_bytes(&self) -> usize {
        self.items.iter().fold(
            size_of::<Self>() + self.label.len() + self.description.len(),
            |bytes, item| bytes.saturating_add(item.storage_bytes()),
        )
    }
}
