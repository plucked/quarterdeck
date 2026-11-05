use std::sync::Arc;

use gpui::{App, ElementId, IntoElement, RenderOnce, Window, div, prelude::*};
use qc_markdown::MarkdownDocument;

use super::block_rendering::render_blocks;

/// A rendered Markdown document, sized to its container's width.
///
/// Built each frame by its parent view, which parses the source once and
/// keeps the document in an `Arc`: elements must own what they draw, so each
/// frame clones the `Arc` instead of the document. `id` must be unique among
/// its siblings; it scopes the ids of the clickable texts inside.
#[derive(IntoElement)]
pub struct MarkdownView {
    id: ElementId,
    document: Arc<MarkdownDocument>,
}

impl MarkdownView {
    pub fn new(id: impl Into<ElementId>, document: Arc<MarkdownDocument>) -> Self {
        Self {
            id: id.into(),
            document,
        }
    }
}

impl RenderOnce for MarkdownView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut next_text_id = 0;
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap_3()
            .children(render_blocks(self.document.blocks(), &mut next_text_id))
    }
}
