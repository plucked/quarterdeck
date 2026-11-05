//! A vertical scrollbar for any gpui scroll container, which gpui does not
//! draw itself: a thumb sized to the visible share of the content, that can
//! be dragged, and a track that jumps to where it is clicked.
//!
//! Files:
//! - `vertical_scrollbar.rs`: `VerticalScrollbar`, the component; paints the thumb and handles the mouse.
//! - `thumb_geometry.rs`: `ThumbGeometry`, where the thumb sits for a scroll position, and back.
//! - `thumb_drag.rs`: `ThumbDrag`, the drag state kept across frames.
//! - `scrollbar_target.rs`: `ScrollbarTarget`, a scroll handle or a virtualized list.

mod scrollbar_target;
mod thumb_drag;
mod thumb_geometry;
mod vertical_scrollbar;

pub use scrollbar_target::ScrollbarTarget;
pub use vertical_scrollbar::VerticalScrollbar;
