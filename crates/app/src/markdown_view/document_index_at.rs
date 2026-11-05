use gpui::{Pixels, Point};

use super::text_segment::TextSegment;

/// The document offset under `position`, for pressing and dragging.
///
/// A segment containing the point wins (so table cells side by side work).
/// Otherwise the point is in a gap or margin: it maps to the first segment
/// that ends below it, at its start when the point is above it, so dragging
/// past the top or into the space between blocks still extends the
/// selection. Below every segment it maps to the end of the last. `None`
/// only for a document without text.
pub fn document_index_at(segments: &[TextSegment], position: Point<Pixels>) -> Option<usize> {
    if let Some(segment) = segments
        .iter()
        .find(|segment| segment.layout.bounds().contains(&position))
    {
        return Some(segment.index_at(position));
    }

    // In a gap: the next segment down decides.
    if let Some(segment) = segments
        .iter()
        .find(|segment| position.y <= segment.layout.bounds().bottom())
    {
        let is_above = position.y < segment.layout.bounds().top();
        return Some(if is_above {
            segment.start
        } else {
            segment.index_at(position)
        });
    }
    segments.last().map(TextSegment::end)
}
