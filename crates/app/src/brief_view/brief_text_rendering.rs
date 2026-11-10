use std::sync::Arc;

use gpui::{AnyElement, Div, IntoElement, SharedString, div, prelude::*};
use qd_markdown::MarkdownDocument;

use crate::markdown_view::MarkdownView;
use crate::theme;

/// Markdown prose of the brief, selectable, links opening only on the web.
pub(super) fn markdown(id: String, document: &Arc<MarkdownDocument>) -> AnyElement {
    div()
        .text_sm()
        .child(MarkdownView::new(SharedString::from(id), document.clone()))
        .into_any_element()
}

/// The caption above code, saying what to see in it.
pub(super) fn caption_line(caption: &str) -> Div {
    div()
        .text_sm()
        .text_color(theme::muted())
        .child(caption.to_owned())
}
