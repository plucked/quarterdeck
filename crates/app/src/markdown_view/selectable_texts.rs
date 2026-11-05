use std::ops::Range;

use gpui::{AnyElement, IntoElement, SharedString, StyledText};
use qd_markdown::{InlineText, LinkRange, StyledRun};

use super::highlight_span::HighlightSpan;
use super::inline_highlight::inline_highlight;
use super::text_segment::{SEGMENT_SEPARATOR, TextSegment};

/// Collects a document's texts while it renders: places each in the
/// document's text, highlights its part of the selection, and keeps its
/// layout so mouse handling can map positions to offsets afterwards. Also
/// hands out element ids unique within one `MarkdownView`.
pub struct SelectableTexts {
    selected: Option<Range<usize>>,
    next_offset: usize,
    next_element_id: usize,
    segments: Vec<TextSegment>,
}

impl SelectableTexts {
    /// Starts a render that highlights `selected` (document offsets).
    pub fn new(selected: Option<Range<usize>>) -> Self {
        Self {
            selected,
            next_offset: 0,
            next_element_id: 0,
            segments: Vec::new(),
        }
    }

    /// An id for an element that needs one (a scrollable code block).
    pub fn next_element_id(&mut self) -> usize {
        let id = self.next_element_id;
        self.next_element_id += 1;
        id
    }

    /// Renders Markdown inline text with its styles.
    pub fn render_inline(&mut self, text: &InlineText) -> AnyElement {
        self.render(text.text(), text.runs(), text.links())
    }

    /// Renders text without styles (code).
    pub fn render_plain(&mut self, text: &str) -> AnyElement {
        self.render(text, &[], &[])
    }

    /// The texts rendered, in document order.
    pub fn into_segments(self) -> Vec<TextSegment> {
        self.segments
    }

    /// Places the text after the previous one, highlights it and records its segment.
    fn render(&mut self, text: &str, runs: &[StyledRun], links: &[LinkRange]) -> AnyElement {
        let start = self.next_offset;
        let shared: SharedString = text.to_owned().into();

        // The selection in this text's own byte offsets; edges outside it are clamped.
        let local_selection = self.selected.as_ref().map(|selected| {
            selected.start.saturating_sub(start)..selected.end.saturating_sub(start)
        });
        let highlights = HighlightSpan::split(text.len(), runs, local_selection.as_ref())
            .into_iter()
            .map(|span| (span.range, inline_highlight(span.style, span.is_selected)));
        let styled = StyledText::new(shared.clone()).with_highlights(highlights);

        self.segments.push(TextSegment {
            start,
            text: shared,
            links: links.to_vec(),
            layout: styled.layout().clone(),
        });
        self.next_offset = start + text.len() + SEGMENT_SEPARATOR.len();
        styled.into_any_element()
    }
}
