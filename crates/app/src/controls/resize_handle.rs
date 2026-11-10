//! `ResizeHandle`: the divider between two panels, dragged to make one of
//! them wider or narrower, like the line right of the sidebar.

use std::rc::Rc;

use gpui::{
    App, Bounds, CursorStyle, DispatchPhase, ElementId, Entity, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, RenderOnce, Window, canvas, div,
    prelude::*, px,
};

use crate::theme;

/// Width of the strip that takes the drag.
const HANDLE_WIDTH: f32 = 5.;
/// Width of the divider line drawn in the strip's middle.
const DIVIDER_WIDTH: f32 = 1.;
/// Width of the line while hovered or dragged.
const ACTIVE_DIVIDER_WIDTH: f32 = 2.;

/// What a new width does.
type ResizeHandler = Rc<dyn Fn(f32, &mut Window, &mut App)>;

/// Which side of the handle the resized panel is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    /// The panel is left of the handle: dragging right widens it (a sidebar).
    Right,
    /// The panel is right of the handle: dragging left widens it (a side panel).
    Left,
}

/// The drag state of one handle, kept by gpui across frames.
#[derive(Default)]
struct ResizeDrag {
    /// While dragged: the pointer's x and the panel's width at the press.
    start: Option<(Pixels, f32)>,
}

/// The divider between a panel and its neighbour in a flex row, as a
/// strip of its own (so it overlaps neither, scrollbars included): place it
/// between them instead of a border. While dragged it reports each new
/// width, kept between `min` and `max`; the owner stores it and redraws.
/// `id` must be unique among its siblings.
#[derive(IntoElement)]
pub struct ResizeHandle {
    id: ElementId,
    edge: ResizeEdge,
    width: f32,
    min: f32,
    max: f32,
    on_resize: ResizeHandler,
}

impl ResizeHandle {
    /// A handle on `edge` of a panel now `width` wide.
    pub fn new(
        id: impl Into<ElementId>,
        edge: ResizeEdge,
        width: f32,
        min: f32,
        max: f32,
        on_resize: impl Fn(f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            edge,
            width,
            min,
            max,
            on_resize: Rc::new(on_resize),
        }
    }
}

impl RenderOnce for ResizeHandle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let drag = window.use_keyed_state(self.id.clone(), cx, |_, _| ResizeDrag::default());
        let is_dragging = drag.read(cx).start.is_some();
        let (line_width, line_color) = if is_dragging {
            (ACTIVE_DIVIDER_WIDTH, theme::accent())
        } else {
            (DIVIDER_WIDTH, theme::border())
        };
        div()
            .id(self.id.clone())
            .group("resize-handle")
            .relative()
            .flex_none()
            .h_full()
            .w(px(HANDLE_WIDTH))
            .flex()
            .justify_center()
            .cursor(CursorStyle::ResizeLeftRight)
            .bg(theme::background())
            .child(
                div()
                    .h_full()
                    .w(px(line_width))
                    .bg(line_color)
                    .group_hover("resize-handle", |line| {
                        line.w(px(ACTIVE_DIVIDER_WIDTH)).bg(theme::accent())
                    }),
            )
            .child(
                drag_canvas(self, drag)
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            )
    }
}

/// Listens window-wide for a press on the strip, the drag that follows and
/// the release, so a drag continues off the strip.
fn drag_canvas(handle: ResizeHandle, drag: Entity<ResizeDrag>) -> gpui::Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds: Bounds<Pixels>, (), window, cx| {
            // Keep the resize cursor for the whole drag, wherever the pointer is.
            if drag.read(cx).start.is_some() {
                window.set_window_cursor_style(CursorStyle::ResizeLeftRight);
            }

            // A press on the strip starts a drag from the current width.
            let pressing = drag.clone();
            let width = handle.width;
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble
                    || event.button != MouseButton::Left
                    || !bounds.contains(&event.position)
                {
                    return;
                }
                pressing.update(cx, |drag, _| drag.start = Some((event.position.x, width)));
                cx.stop_propagation();
                window.refresh();
            });

            // While dragged, the width follows the pointer within its limits.
            let moving = drag.clone();
            let (edge, min, max) = (handle.edge, handle.min, handle.max);
            let on_resize = handle.on_resize.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                let Some((start_x, start_width)) = moving.read(cx).start else {
                    return;
                };
                if phase != DispatchPhase::Bubble || event.pressed_button != Some(MouseButton::Left)
                {
                    return;
                }
                let moved = f32::from(event.position.x - start_x);
                let grown = match edge {
                    ResizeEdge::Right => moved,
                    ResizeEdge::Left => -moved,
                };
                on_resize((start_width + grown).clamp(min, max), window, cx);
            });

            // A release anywhere ends the drag.
            let releasing = drag.clone();
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble
                    && event.button == MouseButton::Left
                    && releasing.read(cx).start.is_some()
                {
                    releasing.update(cx, |drag, _| drag.start = None);
                    window.refresh();
                }
            });
        },
    )
}
