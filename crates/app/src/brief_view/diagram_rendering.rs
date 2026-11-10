use gpui::{Div, IntoElement, Rgba, Styled, canvas, div, px};
use qd_studio::{CodeAnchor, NodeRole};

use super::brief_handlers::BriefHandlers;
use super::diagram_geometry::DiagramRect;
use super::diagram_stroke::{DiagramStroke, paint_strokes};
use crate::theme;

/// Width of the role stripe at a box's left edge (a lane header's top).
pub(super) const ROLE_STRIPE: f32 = 3.;
/// Dash and gap of async arrows and replies.
pub(super) const DASHED: (f32, f32) = (5., 4.);
/// Dash and gap of optional arrows and lifelines.
pub(super) const DOTTED: (f32, f32) = (2., 3.);

/// A canvas filling its diagram that paints `strokes` behind the boxes.
pub(super) fn stroke_canvas(strokes: Vec<DiagramStroke>) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| paint_strokes(bounds, &strokes, window),
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// A div placed at `rect` inside its diagram.
pub(super) fn placed(rect: DiagramRect) -> Div {
    div()
        .absolute()
        .left(px(rect.x))
        .top(px(rect.y))
        .w(px(rect.width))
        .h(px(rect.height))
}

/// A click handler opening `anchor`'s lines.
pub(super) fn anchor_click(
    anchor: &CodeAnchor,
    handlers: &BriefHandlers,
) -> impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static {
    let target = anchor.clone();
    let show_anchor = handlers.show_anchor.clone();
    move |_, window, cx| show_anchor(&target, window, cx)
}

/// The color standing for a role, as a second cue next to the label:
/// orange for a state owner, blue for a transport, teal for stored state,
/// purple for an outside system, olive for a constraint, green for an actor.
pub(super) fn role_color(role: NodeRole) -> Rgba {
    let colors = theme::label_colors();
    match role {
        NodeRole::Transport => colors[0],
        NodeRole::External => colors[1],
        NodeRole::Store => colors[2],
        NodeRole::Owner => colors[3],
        NodeRole::Actor => colors[5],
        NodeRole::Constraint => colors[6],
    }
}
