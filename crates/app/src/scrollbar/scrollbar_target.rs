use gpui::{ListState, Pixels, ScrollHandle, point, px};

/// What a scrollbar scrolls: a scroll container's handle, or a virtualized
/// list's state, which knows its heights only for the rows it has measured.
#[derive(Clone)]
pub enum ScrollbarTarget {
    Handle(ScrollHandle),
    List(ListState),
}

impl ScrollbarTarget {
    /// Height of the visible area.
    pub fn viewport_height(&self) -> Pixels {
        match self {
            Self::Handle(handle) => handle.bounds().size.height,
            Self::List(list) => list.viewport_bounds().size.height,
        }
    }

    /// How far the content can scroll down.
    pub fn max_scroll(&self) -> Pixels {
        match self {
            Self::Handle(handle) => handle.max_offset().height,
            Self::List(list) => list.max_offset_for_scrollbar().height,
        }
    }

    /// How far the content is scrolled down, 0 at the top.
    pub fn scroll_top(&self) -> Pixels {
        // gpui's offsets are negative as the content moves up.
        match self {
            Self::Handle(handle) => -handle.offset().y,
            Self::List(list) => -list.scroll_px_offset_for_scrollbar().y,
        }
    }

    /// Scrolls so the content is `scroll_top` down.
    pub fn set_scroll_top(&self, scroll_top: Pixels) {
        match self {
            Self::Handle(handle) => {
                let offset = handle.offset();
                handle.set_offset(point(offset.x, -scroll_top));
            }
            Self::List(list) => list.set_offset_from_scrollbar(point(px(0.), -scroll_top)),
        }
    }

    /// A thumb drag starts: a list keeps its scroll range fixed until it ends,
    /// so rows measured meanwhile do not move the thumb away from the pointer.
    pub fn begin_drag(&self) {
        if let Self::List(list) = self {
            list.scrollbar_drag_started();
        }
    }

    pub fn end_drag(&self) {
        if let Self::List(list) = self {
            list.scrollbar_drag_ended();
        }
    }
}
