use crate::{ColumnAlignment, InlineText};

/// A GitHub-style table. Rows may have fewer cells than there are columns;
/// the renderer leaves the missing ones empty.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkdownTable {
    pub alignments: Vec<ColumnAlignment>,
    pub header: Vec<InlineText>,
    pub rows: Vec<Vec<InlineText>>,
}
