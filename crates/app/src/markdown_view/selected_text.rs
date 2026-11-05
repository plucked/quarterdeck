use std::ops::Range;

use super::text_segment::{SEGMENT_SEPARATOR, TextSegment};

/// The text of `segments` within the document range `selected`, as copied:
/// each segment's part, joined by [`SEGMENT_SEPARATOR`].
pub fn selected_text(segments: &[TextSegment], selected: Range<usize>) -> String {
    segments
        .iter()
        .filter(|segment| segment.start < selected.end && selected.start <= segment.end())
        .map(|segment| {
            let local_start = selected.start.saturating_sub(segment.start);
            let local_end = (selected.end - segment.start).min(segment.text.len());
            segment.text.get(local_start..local_end).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(SEGMENT_SEPARATOR)
}

#[cfg(test)]
mod tests {
    use gpui::TextLayout;

    use super::*;

    /// Segments laid out as the renderer places them: one separator apart.
    fn segments(texts: &[&str]) -> Vec<TextSegment> {
        let mut start = 0;
        texts
            .iter()
            .map(|text| {
                let segment = TextSegment {
                    start,
                    text: (*text).to_owned().into(),
                    links: Vec::new(),
                    layout: TextLayout::default(),
                };
                start += text.len() + SEGMENT_SEPARATOR.len();
                segment
            })
            .collect()
    }

    /// A selection inside one segment copies just that part.
    #[test]
    fn copies_within_one_segment() {
        let segments = segments(&["hello world"]);
        assert_eq!(selected_text(&segments, 6..11), "world");
    }

    /// A selection across segments joins their parts with line breaks.
    #[test]
    fn copies_across_segments() {
        // "first" is 0..5, "second" 6..12, "third" 13..18.
        let segments = segments(&["first", "second", "third"]);
        assert_eq!(selected_text(&segments, 2..15), "rst\nsecond\nth");
    }
}
