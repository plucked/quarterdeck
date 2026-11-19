//! GitHub-flavored Markdown parsed into a typed document: blocks with
//! flattened, styled inline text, so a UI can render it without knowing
//! Markdown. UI-free; the app's `markdown_view` renders it with gpui.
//!
//! Files:
//! - `rich_document.rs`: styled draft text and mappings to the source.
//! - `rich_blocks.rs`: source ranges of list items, block prefixes and fences.
//! - `container_continuation.rs`: continues the indentation and quote markers of a block.
//! - `fence_delimiters.rs`: emits matching fences when a code payload changes.
//! - `markdown_fence.rs`: delimited code and suggestion payloads.
//! - `block_format.rs`: editable Markdown block prefix kinds.
//! - `inline_mark.rs`: inline delimiters and their original source ranges.
//! - `markdown_document.rs`: `MarkdownDocument`, the parse entry point and its blocks.
//! - `document_builder.rs`: turns pulldown-cmark's events into blocks.
//! - `markdown_block.rs`: `MarkdownBlock`, one paragraph, heading, code block, quote, list, table or rule.
//! - `inline_text.rs`: `InlineText`, flattened text with its styled runs and links.
//! - `inline_style.rs`: `InlineStyle`, the combinable styles of a run.
//! - `styled_run.rs`: `StyledRun`, a styled byte range.
//! - `link_range.rs`: `LinkRange`, a linked byte range and its URL.
//! - `heading_level.rs`: `HeadingLevel`, one to six.
//! - `markdown_list.rs`: `MarkdownList`, a bulleted or numbered list.
//! - `list_item.rs`: `ListItem`, an item's task checkbox and blocks.
//! - `task_state.rs`: `TaskState`, open or done.
//! - `markdown_table.rs`: `MarkdownTable`, header, rows and column alignments.
//! - `column_alignment.rs`: `ColumnAlignment` of a table column.
//! - `html_text.rs`: extracts readable text from raw HTML.
//! - `bare_urls.rs`: finds bare web URLs in text, which GitHub links.

mod bare_urls;
mod block_format;
mod column_alignment;
mod container_continuation;
mod document_builder;
mod fence_delimiters;
mod heading_level;
mod html_text;
mod inline_mark;
mod inline_style;
mod inline_text;
mod link_range;
mod list_item;
mod markdown_block;
mod markdown_document;
mod markdown_fence;
mod markdown_list;
mod markdown_table;
mod rich_blocks;
mod rich_document;
mod styled_run;
mod task_state;

pub use block_format::BlockFormat;
pub use column_alignment::ColumnAlignment;
pub use container_continuation::container_continuation;
pub use fence_delimiters::FenceDelimiters;
pub use heading_level::HeadingLevel;
pub use inline_mark::InlineMark;
pub use inline_style::InlineStyle;
pub use inline_text::InlineText;
pub use link_range::LinkRange;
pub use list_item::ListItem;
pub use markdown_block::MarkdownBlock;
pub use markdown_document::MarkdownDocument;
pub use markdown_fence::MarkdownFence;
pub use markdown_list::MarkdownList;
pub use markdown_table::MarkdownTable;
pub use rich_blocks::RichBlocks;
pub use rich_document::RichDocument;
pub use styled_run::StyledRun;
pub use task_state::TaskState;
