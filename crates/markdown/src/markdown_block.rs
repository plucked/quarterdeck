use crate::{HeadingLevel, InlineText, MarkdownList, MarkdownTable};

/// One block of a Markdown document.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownBlock {
    Paragraph(InlineText),
    Heading {
        level: HeadingLevel,
        text: InlineText,
    },
    /// A fenced or indented code block; `language` is the first word of a
    /// fence's info string, when given.
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    BlockQuote(Vec<MarkdownBlock>),
    List(MarkdownList),
    Table(MarkdownTable),
    /// A thematic break (`---`).
    Rule,
}
