use super::walkthrough_item::WalkthroughItem;

/// A saved bucket mapped to the currently loaded files, plus its progress.
pub(super) struct WalkthroughBucket {
    pub order: u32,
    pub label: String,
    pub description: String,
    pub files: Vec<usize>,
    pub items: Vec<WalkthroughItem>,
    pub remaining: usize,
    pub viewed: usize,
}

impl WalkthroughBucket {
    /// Shows unfinished items and viewed files across the full bucket.
    pub fn progress_label(&self) -> String {
        format!(
            "{} of {} items left · {}/{} files viewed",
            self.remaining,
            self.items.len(),
            self.viewed,
            self.files.len()
        )
    }
}
