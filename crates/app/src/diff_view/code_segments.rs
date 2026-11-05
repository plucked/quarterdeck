use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Pixels, Point, TextLayout};

use super::{DiffPosition, DiffSide};

/// One code text drawn this frame: which row and column it shows, and its
/// laid-out text for mapping the mouse to a byte.
#[derive(Clone)]
pub struct CodeSegment {
    pub row: usize,
    pub side: DiffSide,
    pub layout: TextLayout,
    pub text_length: usize,
}

/// The code texts painted in the current frame, for mouse hit testing.
///
/// Cleared when the detail view renders and filled while its rows paint:
/// only painted texts register, so every layout here has its bounds (rows
/// that were only measured never do). Shared by the view and its rows'
/// paint callbacks within the main thread, hence `Rc`.
#[derive(Clone, Default)]
pub struct CodeSegments(Rc<RefCell<Vec<CodeSegment>>>);

impl CodeSegments {
    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }

    pub fn register(&self, segment: CodeSegment) {
        self.0.borrow_mut().push(segment);
    }

    /// The column and position of the code text under `point`, if any.
    pub fn hit(&self, point: Point<Pixels>) -> Option<(DiffSide, DiffPosition)> {
        let segments = self.0.borrow();
        let segment = segments
            .iter()
            .find(|segment| segment.layout.bounds().contains(&point))?;
        Some((segment.side, position_in(segment, point)))
    }

    /// The position in column `side` nearest to `point`, for extending a
    /// drag: the text on the same line, the start of the first text below
    /// a point above every text, or the end of the last one below them all.
    pub fn nearest_on_side(&self, side: DiffSide, point: Point<Pixels>) -> Option<DiffPosition> {
        let segments = self.0.borrow();
        let mut on_side: Vec<&CodeSegment> = segments
            .iter()
            .filter(|segment| segment.side == side)
            .collect();
        on_side.sort_by_key(|segment| segment.row);

        // The text whose lines span the point's height, else the next one down.
        if let Some(segment) = on_side.iter().find(|segment| {
            let bounds = segment.layout.bounds();
            bounds.top() <= point.y && point.y <= bounds.bottom()
        }) {
            return Some(position_in(segment, point));
        }
        if let Some(below) = on_side
            .iter()
            .find(|segment| point.y < segment.layout.bounds().top())
        {
            return Some(DiffPosition {
                row: below.row,
                byte: 0,
            });
        }
        on_side.last().map(|last| DiffPosition {
            row: last.row,
            byte: last.text_length,
        })
    }
}

/// The byte of `segment`'s text nearest to `point`.
fn position_in(segment: &CodeSegment, point: Point<Pixels>) -> DiffPosition {
    let byte = match segment.layout.index_for_position(point) {
        Ok(index) | Err(index) => index,
    };
    DiffPosition {
        row: segment.row,
        byte: byte.min(segment.text_length),
    }
}
