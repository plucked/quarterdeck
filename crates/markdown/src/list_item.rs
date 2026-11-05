use crate::{MarkdownBlock, TaskState};

/// One item of a list: an optional task checkbox and the item's blocks
/// (paragraphs, nested lists, code).
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub task: Option<TaskState>,
    pub blocks: Vec<MarkdownBlock>,
}
