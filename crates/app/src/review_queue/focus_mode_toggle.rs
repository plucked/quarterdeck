//! The window's Focus Mode button and its framing-corner icon.

use gpui::{Context, IntoElement, PathBuilder, canvas, div, point, prelude::*, px};

use super::ReviewQueue;
use crate::controls::TextTooltip;
use crate::theme;

const BUTTON_SIZE: f32 = 26.;
const ICON_SIZE: f32 = 16.;
const ICON_STROKE: f32 = 1.5;
const CORNER_LENGTH: f32 = 4.;
const CORNER_INSET: f32 = 2.;

impl ReviewQueue {
    /// Enters Focus Mode from the toolbar or leaves from the focused view.
    pub(super) fn render_focus_mode_toggle(
        is_focused: bool,
        is_disabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let label = if is_focused {
            "Exit Focus Mode (⇧⌘F)"
        } else {
            "Focus Mode (⇧⌘F)"
        };
        div()
            .id("focus-mode-toggle")
            .debug_selector(|| "focus-mode-toggle".into())
            .w(px(BUTTON_SIZE))
            .when(is_focused, |button| button.h(px(BUTTON_SIZE)))
            .when(!is_focused, Styled::h_full)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .when(is_focused, |button| button.bg(theme::surface()))
            .when(is_disabled, |button| button.opacity(0.35))
            .when(!is_disabled, |button| {
                button
                    .cursor_pointer()
                    .hover(|style| style.bg(theme::hover()))
                    .tooltip(TextTooltip::builder(label))
                    .on_click(cx.listener(|queue, _, window, cx| {
                        cx.stop_propagation();
                        queue.toggle_focus_mode(window, cx);
                    }))
            })
            .child(render_focus_icon(is_focused))
    }
}

/// Four corners frame the review. In Focus Mode they point inward for exit.
fn render_focus_icon(is_focused: bool) -> impl IntoElement {
    let color = theme::muted();
    let corner_offset = if is_focused {
        CORNER_INSET + CORNER_LENGTH
    } else {
        CORNER_INSET
    };
    let arm = if is_focused {
        -CORNER_LENGTH
    } else {
        CORNER_LENGTH
    };
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let mut corners = PathBuilder::stroke(px(ICON_STROKE));
            for horizontal in [1., -1.] {
                for vertical in [1., -1.] {
                    let corner = bounds.center()
                        - point(
                            px(horizontal * (ICON_SIZE / 2. - corner_offset)),
                            px(vertical * (ICON_SIZE / 2. - corner_offset)),
                        );
                    corners.move_to(corner + point(px(arm * horizontal), px(0.)));
                    corners.line_to(corner);
                    corners.line_to(corner + point(px(0.), px(arm * vertical)));
                }
            }
            if let Ok(path) = corners.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(ICON_SIZE))
}
