use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::html_text::{is_line_break_tag, strip_html};
use crate::{
    ColumnAlignment, HeadingLevel, InlineStyle, InlineText, ListItem, MarkdownBlock, MarkdownList,
    MarkdownTable, TaskState,
};

/// Text shown for an image without alt text; images render as links.
const IMAGE_PLACEHOLDER: &str = "image";

/// Which row of a table ended.
#[derive(Clone, Copy)]
enum TableRowKind {
    Header,
    Body,
}

/// A block that holds other blocks (or list items, or table cells) while its
/// end has not been seen yet.
enum Container {
    Root(Vec<MarkdownBlock>),
    BlockQuote(Vec<MarkdownBlock>),
    List {
        start: Option<u64>,
        items: Vec<ListItem>,
    },
    Item {
        task: Option<TaskState>,
        blocks: Vec<MarkdownBlock>,
    },
    Table {
        alignments: Vec<ColumnAlignment>,
        header: Vec<InlineText>,
        rows: Vec<Vec<InlineText>>,
        current_row: Vec<InlineText>,
    },
}

impl Container {
    /// The finished block for a closed container; `None` for the root and
    /// for items, which their list collects instead.
    fn into_block(self) -> Option<MarkdownBlock> {
        match self {
            Self::BlockQuote(blocks) => Some(MarkdownBlock::BlockQuote(blocks)),
            Self::List { start, items } => Some(MarkdownBlock::List(MarkdownList { start, items })),
            Self::Table {
                alignments,
                header,
                rows,
                current_row: _,
            } => Some(MarkdownBlock::Table(MarkdownTable {
                alignments,
                header,
                rows,
            })),
            Self::Root(_) | Self::Item { .. } => None,
        }
    }

    /// The block list new blocks go into, for containers that hold blocks.
    fn blocks_mut(&mut self) -> Option<&mut Vec<MarkdownBlock>> {
        match self {
            Self::Root(blocks) | Self::BlockQuote(blocks) | Self::Item { blocks, .. } => {
                Some(blocks)
            }
            Self::List { .. } | Self::Table { .. } => None,
        }
    }
}

/// What the inline text being collected becomes when it ends.
enum InlineTarget {
    Paragraph,
    Heading(HeadingLevel),
    TableCell,
    /// Text of a tight list item, which has no paragraph events of its own.
    ImplicitParagraph,
}

/// A link or image whose end has not been seen yet.
struct OpenLink {
    start: usize,
    url: String,
    is_image: bool,
}

/// Inline text being collected, with the styles and links open at its end.
struct OpenInline {
    target: InlineTarget,
    text: InlineText,
    styles: Vec<InlineStyle>,
    links: Vec<OpenLink>,
}

impl OpenInline {
    fn new(target: InlineTarget) -> Self {
        Self {
            target,
            text: InlineText::default(),
            styles: Vec::new(),
            links: Vec::new(),
        }
    }

    /// All styles open right now, combined.
    fn current_style(&self) -> InlineStyle {
        self.styles
            .iter()
            .fold(InlineStyle::PLAIN, |combined, style| combined | *style)
    }
}

/// Turns pulldown-cmark's event stream into [`MarkdownBlock`]s.
///
/// Containers (quotes, lists, items, tables) are a stack; inline text is
/// collected into one [`OpenInline`] at a time. Events that do not fit where
/// they arrive go to the nearest container that takes blocks, so malformed
/// input still renders.
pub(crate) struct DocumentBuilder {
    /// Never empty: the root sits at the bottom.
    containers: Vec<Container>,
    inline: Option<OpenInline>,
    code_block: Option<(Option<String>, String)>,
    html_block: Option<String>,
}

impl DocumentBuilder {
    /// Parses `source` as GitHub-flavored Markdown (tables, strikethrough,
    /// task lists) into blocks.
    pub(crate) fn build(source: &str) -> Vec<MarkdownBlock> {
        let options =
            Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
        let mut builder = Self {
            containers: vec![Container::Root(Vec::new())],
            inline: None,
            code_block: None,
            html_block: None,
        };
        for event in Parser::new_ext(source, options) {
            builder.handle(event);
        }

        // Text of a final tight list item may still be open.
        builder.close_implicit_paragraph();
        match builder.containers.swap_remove(0) {
            Container::Root(blocks) => blocks,
            other => other.into_block().into_iter().collect(),
        }
    }

    fn handle(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(code) | Event::InlineMath(code) | Event::DisplayMath(code) => {
                self.push_inline(&code, InlineStyle::CODE);
            }
            Event::Html(html) => self.html(&html),
            Event::InlineHtml(html) => self.inline_html(&html),
            Event::FootnoteReference(label) => {
                self.push_inline(&format!("[^{label}]"), InlineStyle::PLAIN);
            }
            Event::SoftBreak => self.push_inline(" ", InlineStyle::PLAIN),
            Event::HardBreak => self.push_inline("\n", InlineStyle::PLAIN),
            Event::Rule => {
                self.close_implicit_paragraph();
                self.push_block(MarkdownBlock::Rule);
            }
            Event::TaskListMarker(is_done) => self.mark_task(is_done),
        }
    }

    /// Opens the container, inline text or style a start tag begins.
    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.open_inline(InlineTarget::Paragraph),
            Tag::Heading { level, .. } => {
                self.open_inline(InlineTarget::Heading(HeadingLevel::from_parser(level)));
            }
            Tag::BlockQuote(_) => self.open_container(Container::BlockQuote(Vec::new())),
            Tag::CodeBlock(kind) => {
                self.close_implicit_paragraph();
                self.code_block = Some((code_language(&kind), String::new()));
            }
            Tag::HtmlBlock => {
                self.close_implicit_paragraph();
                self.html_block = Some(String::new());
            }
            Tag::List(start) => self.open_container(Container::List {
                start,
                items: Vec::new(),
            }),
            Tag::Item => self.open_container(Container::Item {
                task: None,
                blocks: Vec::new(),
            }),
            Tag::Table(alignments) => self.open_container(Container::Table {
                alignments: alignments
                    .into_iter()
                    .map(ColumnAlignment::from_parser)
                    .collect(),
                header: Vec::new(),
                rows: Vec::new(),
                current_row: Vec::new(),
            }),
            Tag::TableCell => self.inline = Some(OpenInline::new(InlineTarget::TableCell)),
            Tag::Emphasis => self.open_style(InlineStyle::EMPHASIS),
            Tag::Strong => self.open_style(InlineStyle::STRONG),
            Tag::Strikethrough => self.open_style(InlineStyle::STRIKETHROUGH),
            // Not styled, but opened so their end has a style to close.
            Tag::Superscript | Tag::Subscript => self.open_style(InlineStyle::PLAIN),
            Tag::Link { dest_url, .. } => self.open_link(dest_url.into_string(), false),
            Tag::Image { dest_url, .. } => self.open_link(dest_url.into_string(), true),
            // Their content flows into the surrounding blocks.
            Tag::TableHead
            | Tag::TableRow
            | Tag::FootnoteDefinition(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::MetadataBlock(_) => {}
        }
    }

    /// Finishes what an end tag closes and hands it to its parent.
    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) => self.finish_inline_block(),
            TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::Table => self.close_container(),
            TagEnd::Item => self.close_item(),
            TagEnd::CodeBlock => self.finish_code_block(),
            TagEnd::HtmlBlock => self.finish_html_block(),
            TagEnd::TableCell => self.finish_table_cell(),
            TagEnd::TableHead => self.finish_table_row(TableRowKind::Header),
            TagEnd::TableRow => self.finish_table_row(TableRowKind::Body),
            TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript => {
                if let Some(inline) = &mut self.inline {
                    inline.styles.pop();
                }
            }
            TagEnd::Link | TagEnd::Image => self.close_link(),
            TagEnd::FootnoteDefinition
            | TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition
            | TagEnd::MetadataBlock(_) => {}
        }
    }

    /// Text goes into an open code block as is, otherwise into inline text.
    fn text(&mut self, text: &str) {
        match &mut self.code_block {
            Some((_, code)) => code.push_str(text),
            None => self.push_inline(text, InlineStyle::PLAIN),
        }
    }

    /// Raw HTML of a block is collected whole, since a comment can span
    /// several events, and stripped when the block ends.
    fn html(&mut self, html: &str) {
        match &mut self.html_block {
            Some(collected) => collected.push_str(html),
            None => self.inline_html(html),
        }
    }

    /// Inline HTML adds only its line breaks; tags and comments are dropped
    /// and the text between them arrives as ordinary text.
    fn inline_html(&mut self, html: &str) {
        if is_line_break_tag(html) {
            self.push_inline("\n", InlineStyle::PLAIN);
        }
    }

    /// Appends text in the open styles plus `style`, opening an implicit
    /// paragraph when no inline text is open (tight list items).
    fn push_inline(&mut self, text: &str, style: InlineStyle) {
        let inline = self
            .inline
            .get_or_insert_with(|| OpenInline::new(InlineTarget::ImplicitParagraph));
        let combined = inline.current_style() | style;
        inline.text.push(text, combined);
    }

    fn open_inline(&mut self, target: InlineTarget) {
        self.close_implicit_paragraph();
        self.inline = Some(OpenInline::new(target));
    }

    fn open_style(&mut self, style: InlineStyle) {
        self.inline
            .get_or_insert_with(|| OpenInline::new(InlineTarget::ImplicitParagraph))
            .styles
            .push(style);
    }

    fn open_link(&mut self, url: String, is_image: bool) {
        let inline = self
            .inline
            .get_or_insert_with(|| OpenInline::new(InlineTarget::ImplicitParagraph));
        inline.styles.push(InlineStyle::LINK);
        inline.links.push(OpenLink {
            start: inline.text.len(),
            url,
            is_image,
        });
    }

    /// Ends the innermost link: an image without alt text gets a
    /// placeholder, then the whole range is recorded as a link.
    fn close_link(&mut self) {
        let Some(inline) = &mut self.inline else {
            return;
        };
        let Some(link) = inline.links.pop() else {
            return;
        };
        if link.is_image && inline.text.len() == link.start {
            let style = inline.current_style();
            inline.text.push(IMAGE_PLACEHOLDER, style);
        }
        inline.styles.pop();
        let end = inline.text.len();
        inline.text.push_link(link.start..end, link.url);
    }

    /// Ends a paragraph or heading and adds it as a block.
    fn finish_inline_block(&mut self) {
        let Some(open) = self.inline.take() else {
            return;
        };
        let mut text = open.text;
        text.trim_end();
        let block = match open.target {
            InlineTarget::Heading(level) => MarkdownBlock::Heading { level, text },
            InlineTarget::Paragraph | InlineTarget::ImplicitParagraph | InlineTarget::TableCell => {
                if text.is_empty() {
                    return;
                }
                MarkdownBlock::Paragraph(text)
            }
        };
        self.push_block(block);
    }

    /// Ends the text of a tight list item before a block starts or the item ends.
    fn close_implicit_paragraph(&mut self) {
        if matches!(
            self.inline,
            Some(OpenInline {
                target: InlineTarget::ImplicitParagraph,
                ..
            })
        ) {
            self.finish_inline_block();
        }
    }

    fn open_container(&mut self, container: Container) {
        self.close_implicit_paragraph();
        self.containers.push(container);
    }

    /// Ends a quote, list or table and adds it as a block.
    fn close_container(&mut self) {
        self.close_implicit_paragraph();
        if self.containers.len() > 1
            && let Some(block) = self.containers.pop().and_then(Container::into_block)
        {
            self.push_block(block);
        }
    }

    /// Ends a list item and adds it to its list.
    fn close_item(&mut self) {
        self.close_implicit_paragraph();
        if self.containers.len() <= 1 {
            return;
        }
        let Some(Container::Item { task, blocks }) = self.containers.pop() else {
            return;
        };
        match self.containers.last_mut() {
            Some(Container::List { items, .. }) => items.push(ListItem { task, blocks }),
            // An item outside a list keeps its blocks.
            _ => blocks.into_iter().for_each(|block| self.push_block(block)),
        }
    }

    /// Sets the checkbox of the list item being built.
    fn mark_task(&mut self, is_done: bool) {
        if let Some(Container::Item { task, .. }) = self.containers.last_mut() {
            *task = Some(if is_done {
                TaskState::Done
            } else {
                TaskState::Open
            });
        }
    }

    /// Ends a code block; the final newline of its body is not part of the code.
    fn finish_code_block(&mut self) {
        let Some((language, mut code)) = self.code_block.take() else {
            return;
        };
        if code.ends_with('\n') {
            code.pop();
        }
        self.push_block(MarkdownBlock::CodeBlock { language, code });
    }

    /// Ends an HTML block; its readable text, if any, becomes a paragraph.
    fn finish_html_block(&mut self) {
        let Some(html) = self.html_block.take() else {
            return;
        };
        let text = strip_html(&html);
        if !text.is_empty() {
            let mut inline = InlineText::default();
            inline.push(&text, InlineStyle::PLAIN);
            self.push_block(MarkdownBlock::Paragraph(inline));
        }
    }

    /// Ends a table cell and adds it to the row being built.
    fn finish_table_cell(&mut self) {
        let mut cell = self.inline.take().map(|open| open.text).unwrap_or_default();
        cell.trim_end();
        if let Some(Container::Table { current_row, .. }) = self.containers.last_mut() {
            current_row.push(cell);
        }
    }

    /// Ends the header or a body row of the table being built.
    fn finish_table_row(&mut self, kind: TableRowKind) {
        if let Some(Container::Table {
            header,
            rows,
            current_row,
            ..
        }) = self.containers.last_mut()
        {
            let row = std::mem::take(current_row);
            match kind {
                TableRowKind::Header => *header = row,
                TableRowKind::Body => rows.push(row),
            }
        }
    }

    /// Adds a finished block to the innermost container that holds blocks.
    fn push_block(&mut self, block: MarkdownBlock) {
        if let Some(blocks) = self
            .containers
            .iter_mut()
            .rev()
            .find_map(Container::blocks_mut)
        {
            blocks.push(block);
        }
    }
}

/// The first word of a fence's info string (```` ```rust title ````), if any.
fn code_language(kind: &CodeBlockKind<'_>) -> Option<String> {
    match kind {
        CodeBlockKind::Fenced(info) => info.split_whitespace().next().map(str::to_owned),
        CodeBlockKind::Indented => None,
    }
}
