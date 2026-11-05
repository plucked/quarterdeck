use crate::ListItem;

/// A bulleted list, or a numbered one starting at `start`.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkdownList {
    /// The first item's number for a numbered list; `None` for bullets.
    pub start: Option<u64>,
    pub items: Vec<ListItem>,
}
