//! GitHub-flavored Markdown parsed into a typed document: blocks with
//! flattened, styled inline text, so a UI can render it without knowing
//! Markdown. UI-free; the app's `markdown_view` renders it with gpui.
//!
//! Files:
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
mod column_alignment;
mod document_builder;
mod heading_level;
mod html_text;
mod inline_style;
mod inline_text;
mod link_range;
mod list_item;
mod markdown_block;
mod markdown_document;
mod markdown_list;
mod markdown_table;
mod styled_run;
mod task_state;

pub use column_alignment::ColumnAlignment;
pub use heading_level::HeadingLevel;
pub use inline_style::InlineStyle;
pub use inline_text::InlineText;
pub use link_range::LinkRange;
pub use list_item::ListItem;
pub use markdown_block::MarkdownBlock;
pub use markdown_document::MarkdownDocument;
pub use markdown_list::MarkdownList;
pub use markdown_table::MarkdownTable;
pub use styled_run::StyledRun;
pub use task_state::TaskState;
