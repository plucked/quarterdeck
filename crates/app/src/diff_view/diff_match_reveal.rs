use std::cell::Cell;
use std::rc::Rc;

use gpui::App;

/// Applies a measured horizontal offset to the searched file.
type ScrollFile = Box<dyn Fn(f32, &mut App)>;

/// A one-shot request from Find to scroll the shaped match into view.
/// Shared across renders so later user scrolling stays in place.
pub struct DiffMatchReveal {
    pending: Cell<bool>,
    scroll_horizontally: ScrollFile,
}

impl DiffMatchReveal {
    pub fn new(scroll_horizontally: impl Fn(f32, &mut App) + 'static) -> Rc<Self> {
        Rc::new(Self {
            pending: Cell::new(false),
            scroll_horizontally: Box::new(scroll_horizontally),
        })
    }

    pub fn request(&self) {
        self.pending.set(true);
    }

    /// Lets just the first matching cell request scrolling during prepaint.
    pub fn take(&self) -> bool {
        self.pending.replace(false)
    }

    /// Aligns an offscreen occurrence with its clipped code column using the actual shaped bounds.
    pub fn reveal_columns(&self, start: f32, end: f32, width: f32, offset: f32, cx: &mut App) {
        let delta = if start < 0. {
            start
        } else {
            (end - width).max(0.).min(start)
        };
        if delta.abs() >= 1. {
            (self.scroll_horizontally)((offset + delta).max(0.), cx);
        }
    }
}
