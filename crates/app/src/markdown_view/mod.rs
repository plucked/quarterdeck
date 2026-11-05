//! Renders a parsed `qc_markdown::MarkdownDocument` as gpui elements: a
//! reusable component for pull request descriptions, comments and anything
//! else GitHub writes in Markdown.
//!
//! Files:
//! - `markdown_view.rs`: `MarkdownView`, the component.
//! - `block_rendering.rs`: renders headings, code, quotes, lists, tables and rules.
//! - `render_inline_text.rs`: renders styled inline text with clickable links.

mod block_rendering;
mod markdown_view;
mod render_inline_text;

pub use markdown_view::MarkdownView;
