use gpui::{
    App, Bounds, Canvas, DispatchPhase, ElementId, Entity, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, RenderOnce, ScrollHandle, Window, canvas,
    div, fill, point, prelude::*, px, size,
};

use super::thumb_drag::ThumbDrag;
use super::thumb_geometry::ThumbGeometry;
use crate::theme;

/// Width of the bar, including the gap around the thumb.
const SCROLLBAR_WIDTH: f32 = 10.;
/// Gap between the thumb and the bar's edges.
const THUMB_INSET: f32 = 2.;

/// A vertical scrollbar for the scroll container tracked by `handle`. Place
/// it inside a `relative()` wrapper that also holds the container; it
/// overlays the wrapper's right edge. `id` must be unique among its siblings.
///
/// The thumb is painted from the handle at paint time, after the container
/// is laid out, so it never lags a frame behind the content.
#[derive(IntoElement)]
pub struct VerticalScrollbar {
    id: ElementId,
    handle: ScrollHandle,
}

impl VerticalScrollbar {
    pub fn new(id: impl Into<ElementId>, handle: ScrollHandle) -> Self {
        Self {
            id: id.into(),
            handle,
        }
    }
}

impl RenderOnce for VerticalScrollbar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let drag = window.use_keyed_state(self.id, cx, |_, _| ThumbDrag::default());
        div()
            .absolute()
            .top_0()
            .right_0()
            .h_full()
            .w(px(SCROLLBAR_WIDTH))
            .child(thumb_canvas(self.handle, drag).size_full())
    }
}

/// Paints the thumb into the bar's bounds and listens for presses, drags and
/// releases on it. Listens window-wide so a drag continues off the bar.
fn thumb_canvas(handle: ScrollHandle, drag: Entity<ThumbDrag>) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |track: Bounds<Pixels>, (), window, cx| {
            let Some(thumb) = current_thumb(&handle, track) else {
                return;
            };

            // Paint the thumb, brighter while dragged.
            let is_dragging = drag.read(cx).grab_offset.is_some();
            let color = if is_dragging {
                theme::scrollbar_thumb_active()
            } else {
                theme::scrollbar_thumb()
            };
            let thumb_bounds = Bounds::new(
                point(
                    track.origin.x + px(THUMB_INSET),
                    track.origin.y + px(thumb.top),
                ),
                size(track.size.width - px(2. * THUMB_INSET), px(thumb.height)),
            );
            window.paint_quad(fill(thumb_bounds, color).corner_radii(px(SCROLLBAR_WIDTH / 2.)));

            // A press on the bar grabs the thumb, or first jumps it to the pointer.
            let pressing = (handle.clone(), drag.clone());
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                let (handle, drag) = &pressing;
                if phase != DispatchPhase::Bubble
                    || event.button != MouseButton::Left
                    || !track.contains(&event.position)
                {
                    return;
                }
                let pointer_y = f32::from(event.position.y - track.origin.y);
                let grab_offset = if thumb.contains(pointer_y) {
                    pointer_y - thumb.top
                } else {
                    let centered = thumb.height / 2.;
                    scroll_to(handle, thumb.scroll_top_for(pointer_y - centered));
                    centered
                };
                drag.update(cx, |drag, _| drag.grab_offset = Some(grab_offset));
                // The press belongs to the bar, not to the text underneath.
                cx.stop_propagation();
                window.refresh();
            });

            // While grabbed, the thumb follows the pointer.
            let moving = (handle.clone(), drag.clone());
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                let (handle, drag) = &moving;
                let Some(grab_offset) = drag.read(cx).grab_offset else {
                    return;
                };
                if phase != DispatchPhase::Bubble || event.pressed_button != Some(MouseButton::Left)
                {
                    return;
                }
                let thumb_top = f32::from(event.position.y - track.origin.y) - grab_offset;
                scroll_to(handle, thumb.scroll_top_for(thumb_top));
                window.refresh();
            });

            // A release anywhere ends the drag.
            let releasing = drag.clone();
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble
                    && event.button == MouseButton::Left
                    && releasing.read(cx).grab_offset.is_some()
                {
                    releasing.update(cx, |drag, _| drag.grab_offset = None);
                    window.refresh();
                }
            });
        },
    )
}

/// The thumb for the container's current scroll position in `track`.
fn current_thumb(handle: &ScrollHandle, track: Bounds<Pixels>) -> Option<ThumbGeometry> {
    ThumbGeometry::compute(
        track.size.height.into(),
        handle.bounds().size.height.into(),
        handle.max_offset().height.into(),
        // gpui's offset is negative as the content moves up.
        -f32::from(handle.offset().y),
    )
}

/// Scrolls the container so its content is `scroll_top` pixels down.
fn scroll_to(handle: &ScrollHandle, scroll_top: f32) {
    let offset = handle.offset();
    handle.set_offset(point(offset.x, px(-scroll_top)));
}
