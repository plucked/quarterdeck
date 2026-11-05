use gpui::{Pixels, Point, SharedString, TextLayout};
use qd_markdown::LinkRange;

/// What separates two segments in the document's text, and in copied text.
pub const SEGMENT_SEPARATOR: &str = "\n";

/// One rendered text of a document (a paragraph, heading, table cell or code
/// block), placed in the document's text: it starts at `start`, and the next
/// segment starts after a [`SEGMENT_SEPARATOR`].
#[derive(Clone)]
pub struct TextSegment {
    pub start: usize,
    pub text: SharedString,
    /// Links by byte range within `text`.
    pub links: Vec<LinkRange>,
    /// The text's layout from the frame it was rendered in. Read only by
    /// mouse handling, which runs after that frame is painted.
    pub layout: TextLayout,
}

impl TextSegment {
    /// The document offset just past the segment's last byte.
    pub fn end(&self) -> usize {
        self.start + self.text.len()
    }

    /// The document offset nearest to `position` within this segment.
    pub fn index_at(&self, position: Point<Pixels>) -> usize {
        let local = match self.layout.index_for_position(position) {
            Ok(index) | Err(index) => index,
        };
        self.start + local.min(self.text.len())
    }

    /// The URL of the link at document offset `index`, if one is there.
    pub fn link_at(&self, index: usize) -> Option<&str> {
        let local = index.checked_sub(self.start)?;
        self.links
            .iter()
            .find(|link| link.range.contains(&local))
            .map(|link| link.url.as_str())
    }
}
