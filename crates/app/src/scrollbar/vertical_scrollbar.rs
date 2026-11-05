use gpui::{
    App, Bounds, Canvas, DispatchPhase, ElementId, Entity, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, RenderOnce, Window, canvas, div, fill,
    point, prelude::*, px, size,
};

use super::scrollbar_target::ScrollbarTarget;
use super::thumb_drag::ThumbDrag;
use super::thumb_geometry::ThumbGeometry;
use crate::theme;

/// Width of the bar, including the gap around the thumb.
const SCROLLBAR_WIDTH: f32 = 10.;
/// Gap between the thumb and the bar's edges.
const THUMB_INSET: f32 = 2.;

/// A vertical scrollbar for a scroll container or a virtualized list. Place
/// it inside a `relative()` wrapper that also holds the container; it
/// overlays the wrapper's right edge. `id` must be unique among its siblings.
///
/// The thumb is painted from the target at paint time, after the container
/// is laid out, so it never lags a frame behind the content.
#[derive(IntoElement)]
pub struct VerticalScrollbar {
    id: ElementId,
    target: ScrollbarTarget,
}

impl VerticalScrollbar {
    pub fn new(id: impl Into<ElementId>, target: ScrollbarTarget) -> Self {
        Self {
            id: id.into(),
            target,
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
            .child(thumb_canvas(self.target, drag).size_full())
    }
}

/// Paints the thumb into the bar's bounds and listens for presses, drags and
/// releases on it. Listens window-wide so a drag continues off the bar.
fn thumb_canvas(target: ScrollbarTarget, drag: Entity<ThumbDrag>) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |track: Bounds<Pixels>, (), window, cx| {
            let Some(thumb) = current_thumb(&target, track) else {
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
            let pressing = (target.clone(), drag.clone());
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                let (target, drag) = &pressing;
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
                    target.set_scroll_top(px(thumb.scroll_top_for(pointer_y - centered)));
                    centered
                };
                target.begin_drag();
                drag.update(cx, |drag, _| drag.grab_offset = Some(grab_offset));
                // The press belongs to the bar, not to the text underneath.
                cx.stop_propagation();
                window.refresh();
            });

            // While grabbed, the thumb follows the pointer.
            let moving = (target.clone(), drag.clone());
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                let (target, drag) = &moving;
                let Some(grab_offset) = drag.read(cx).grab_offset else {
                    return;
                };
                if phase != DispatchPhase::Bubble || event.pressed_button != Some(MouseButton::Left)
                {
                    return;
                }
                let thumb_top = f32::from(event.position.y - track.origin.y) - grab_offset;
                target.set_scroll_top(px(thumb.scroll_top_for(thumb_top)));
                window.refresh();
            });

            // A release anywhere ends the drag.
            let releasing = (target.clone(), drag.clone());
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                let (target, drag) = &releasing;
                if phase == DispatchPhase::Bubble
                    && event.button == MouseButton::Left
                    && drag.read(cx).grab_offset.is_some()
                {
                    target.end_drag();
                    drag.update(cx, |drag, _| drag.grab_offset = None);
                    window.refresh();
                }
            });
        },
    )
}

/// The thumb for the target's current scroll position in `track`.
fn current_thumb(target: &ScrollbarTarget, track: Bounds<Pixels>) -> Option<ThumbGeometry> {
    ThumbGeometry::compute(
        track.size.height.into(),
        target.viewport_height().into(),
        target.max_scroll().into(),
        target.scroll_top().into(),
    )
}
