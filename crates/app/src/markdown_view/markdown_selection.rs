use std::ops::Range;

use gpui::{Context, FocusHandle, Pixels, Point, Window};

use super::document_index_at::document_index_at;
use super::open_web_link::open_web_link;
use super::selected_text::selected_text;
use super::text_segment::TextSegment;
use super::text_selection::TextSelection;
use crate::word_range_at::word_range_at;

/// How a press selects, by click count.
const SINGLE_CLICK: usize = 1;
const DOUBLE_CLICK: usize = 2;

/// The selection state of one rendered Markdown document, kept by gpui
/// across frames while the document is shown.
///
/// A press places a caret (or selects a word on a double click, the whole
/// text on a triple click), dragging moves the selection's head, and a
/// release without a drag clicks the link under the pointer instead.
pub struct MarkdownSelection {
    /// Focused on press, so the Copy action reaches the document.
    focus: FocusHandle,
    selection: Option<TextSelection>,
    is_dragging: bool,
    /// The texts hit testing reads: those of the last frame that painted
    /// the document, so every layout has its bounds (gpui panics on the
    /// bounds of a layout that was never prepainted).
    segments: Vec<TextSegment>,
    /// The texts of the last render, waiting for the paint that gives
    /// their layouts bounds. A virtualized list renders rows to measure
    /// them without painting them; those texts never reach `segments`.
    rendered: Vec<TextSegment>,
}

impl MarkdownSelection {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            selection: None,
            is_dragging: false,
            segments: Vec::new(),
            rendered: Vec::new(),
        }
    }

    pub fn focus(&self) -> &FocusHandle {
        &self.focus
    }

    pub fn is_dragging(&self) -> bool {
        self.is_dragging
    }

    /// The selected document range; `None` when nothing is selected.
    pub fn selected_range(&self) -> Option<Range<usize>> {
        self.selection
            .as_ref()
            .filter(|selection| !selection.is_empty())
            .map(TextSelection::range)
    }

    /// The selected text as copied; `None` when nothing is selected.
    pub fn selected_text(&self) -> Option<String> {
        self.selected_range()
            .map(|range| selected_text(&self.segments, range))
    }

    /// Keeps the texts of the frame being rendered until it paints them.
    /// Changes nothing that is drawn, so it does not notify.
    pub fn set_rendered_segments(&mut self, segments: Vec<TextSegment>) {
        self.rendered = segments;
    }

    /// Hands the rendered texts to hit testing, now that they are painted
    /// (called from the document's paint). Changes nothing that is drawn.
    pub fn commit_painted_segments(&mut self) {
        if !self.rendered.is_empty() {
            self.segments = std::mem::take(&mut self.rendered);
        }
    }

    /// Starts a selection at `position` for a press with `click_count` clicks.
    pub fn begin(
        &mut self,
        position: Point<Pixels>,
        click_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus);
        let Some(index) = document_index_at(&self.segments, position) else {
            return;
        };

        // One click starts a drag; more select a word or the whole text at once.
        self.selection = Some(match click_count {
            0 | SINGLE_CLICK => TextSelection::caret(index),
            DOUBLE_CLICK => TextSelection::covering(self.word_range_at(index)),
            _ => TextSelection::covering(self.segment_range_at(index)),
        });
        self.is_dragging = click_count <= SINGLE_CLICK;
        cx.notify();
    }

    /// Moves the selection's head to `position` while dragging.
    pub fn extend(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if !self.is_dragging {
            return;
        }
        let Some(index) = document_index_at(&self.segments, position) else {
            return;
        };
        if let Some(selection) = &mut self.selection
            && selection.head != index
        {
            selection.head = index;
            cx.notify();
        }
    }

    /// Ends a drag. A press released where it started selected nothing: it
    /// was a click, which opens the link under it.
    pub fn finish(&mut self, cx: &mut Context<Self>) {
        if !self.is_dragging {
            return;
        }
        self.is_dragging = false;
        let Some(selection) = &self.selection else {
            return;
        };
        if selection.is_empty() {
            let index = selection.head;
            if let Some(url) = self
                .segment_at(index)
                .and_then(|segment| segment.link_at(index))
            {
                open_web_link(url, cx);
            }
            self.selection = None;
        }
        cx.notify();
    }

    /// The segment holding document offset `index`.
    fn segment_at(&self, index: usize) -> Option<&TextSegment> {
        self.segments
            .iter()
            .find(|segment| segment.start <= index && index <= segment.end())
    }

    /// The document range of the word at `index`.
    fn word_range_at(&self, index: usize) -> Range<usize> {
        self.segment_at(index).map_or(index..index, |segment| {
            let local = word_range_at(&segment.text, index - segment.start);
            segment.start + local.start..segment.start + local.end
        })
    }

    /// The document range of the whole text at `index`.
    fn segment_range_at(&self, index: usize) -> Range<usize> {
        self.segment_at(index)
            .map_or(index..index, |segment| segment.start..segment.end())
    }
}
