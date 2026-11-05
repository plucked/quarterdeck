//! Renders a parsed `qd_markdown::MarkdownDocument` as gpui elements: a
//! reusable component for pull request descriptions, comments and anything
//! else GitHub writes in Markdown. Its text can be selected with the mouse
//! (drag, double click for a word, triple click for a block) and copied.
//!
//! Files:
//! - `markdown_view.rs`: `MarkdownView`, the component and its mouse and Copy handling.
//! - `block_rendering.rs`: renders headings, code, quotes, lists, tables and rules.
//! - `selectable_texts.rs`: `SelectableTexts`, places each text in the document's text while rendering.
//! - `text_segment.rs`: `TextSegment`, one rendered text with its offset, links and layout.
//! - `markdown_selection.rs`: `MarkdownSelection`, the selection state kept across frames.
//! - `text_selection.rs`: `TextSelection`, anchor and head in document offsets.
//! - `document_index_at.rs`: maps a mouse position to a document offset.
//! - `selected_text.rs`: the text of a selection, as copied.
//! - `highlight_span.rs`: `HighlightSpan`, cuts styled runs at the selection's edges.
//! - `inline_highlight.rs`: the gpui highlight for a style and selection state.
//! - `open_web_link.rs`: opens a clicked link when it is a web URL.

mod block_rendering;
mod document_index_at;
mod highlight_span;
mod inline_highlight;
mod markdown_selection;
mod markdown_view;
mod open_web_link;
mod selectable_texts;
mod selected_text;
mod text_segment;
mod text_selection;

pub use markdown_view::MarkdownView;
