use std::ops::Range;

use gpui::{Bounds, Pixels, Point, WrappedLine, point, px};

/// A text area's text as shaped for one width: one wrapped line per
/// logical line (split at `\n`), with each line's byte range in the text
/// and its top, so offsets and positions map both ways.
pub struct TextAreaLayout {
    pub lines: Vec<WrappedLine>,
    /// Byte range of each line in the text, without its `\n`.
    pub ranges: Vec<Range<usize>>,
    /// Top of each line, from the text's top.
    pub tops: Vec<Pixels>,
    pub line_height: Pixels,
    pub height: Pixels,
}

impl TextAreaLayout {
    /// The line holding byte `offset` (the last line for the text's end).
    fn line_of(&self, offset: usize) -> usize {
        self.ranges
            .iter()
            .position(|range| offset <= range.end)
            .unwrap_or(self.ranges.len().saturating_sub(1))
    }

    /// Where byte `offset` is drawn, from the text's top left.
    pub fn position_for_offset(&self, offset: usize) -> Point<Pixels> {
        let index = self.line_of(offset);
        let (Some(line), Some(range), Some(top)) = (
            self.lines.get(index),
            self.ranges.get(index),
            self.tops.get(index),
        ) else {
            return point(px(0.), px(0.));
        };
        let within = offset.saturating_sub(range.start).min(range.len());
        let position = line
            .position_for_index(within, self.line_height)
            .unwrap_or_else(|| point(line.width(), px(0.)));
        point(position.x, *top + position.y)
    }

    /// The byte offset nearest `position` (from the text's top left).
    pub fn offset_for_position(&self, position: Point<Pixels>) -> usize {
        let Some(last) = self.lines.len().checked_sub(1) else {
            return 0;
        };
        let index = self
            .tops
            .iter()
            .rposition(|top| *top <= position.y)
            .unwrap_or(0)
            .min(last);
        let (line, range, top) = (&self.lines[index], &self.ranges[index], self.tops[index]);
        let local = point(position.x.max(px(0.)), (position.y - top).max(px(0.)));
        let within = match line.closest_index_for_position(local, self.line_height) {
            Ok(within) | Err(within) => within,
        };
        range.start + within.min(range.len())
    }

    /// The rectangles that cover bytes `range`, one per visual row, inside
    /// a text area of `width`, from the text's top left.
    pub fn selection_rects(&self, range: &Range<usize>, width: Pixels) -> Vec<Bounds<Pixels>> {
        let start = self.position_for_offset(range.start);
        let end = self.position_for_offset(range.end);
        let height = self.line_height;
        if start.y == end.y {
            return vec![Bounds::from_corners(start, point(end.x, end.y + height))];
        }
        let mut rects = vec![Bounds::from_corners(start, point(width, start.y + height))];
        if end.y > start.y + height {
            rects.push(Bounds::from_corners(
                point(px(0.), start.y + height),
                point(width, end.y),
            ));
        }
        rects.push(Bounds::from_corners(
            point(px(0.), end.y),
            point(end.x, end.y + height),
        ));
        rects
    }
}
