/// One proposed reading stop, with exact paths or folder selectors ending in `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemCandidate {
    pub label: String,
    pub description: String,
    /// Reading order: exact paths stay in place; folders expand in frozen inventory order.
    pub paths: Vec<String>,
}

impl ItemCandidate {
    /// Includes empty selectors and item slots in the request resource budget.
    pub(crate) fn storage_bytes(&self) -> usize {
        self.paths.iter().fold(
            size_of::<Self>() + self.label.len() + self.description.len(),
            |bytes, path| bytes.saturating_add(size_of::<String>() + path.len()),
        )
    }
}
