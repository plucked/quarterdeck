use crate::MarkdownBlock;
use crate::document_builder::DocumentBuilder;

/// A parsed Markdown text: its blocks, ready to render.
///
/// Parsing never fails; anything the model does not cover (raw HTML,
/// footnote definitions) degrades to plain text or is dropped.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MarkdownDocument {
    blocks: Vec<MarkdownBlock>,
}

impl MarkdownDocument {
    /// Parses GitHub-flavored Markdown: `CommonMark` plus tables,
    /// strikethrough and task lists. Raw HTML keeps only its text and `<br>`
    /// line breaks; images become links labeled with their alt text.
    pub fn parse(source: &str) -> Self {
        Self {
            blocks: DocumentBuilder::build(source),
        }
    }

    pub fn blocks(&self) -> &[MarkdownBlock] {
        &self.blocks
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}
