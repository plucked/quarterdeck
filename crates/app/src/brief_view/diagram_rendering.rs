use gpui::{Canvas, Div, IntoElement, Rgba, Styled, canvas, div, px};
use qd_studio::NodeRole;

use super::diagram_geometry::DiagramRect;
use super::diagram_stroke::{DiagramStroke, paint_strokes};
use crate::theme;

/// Width of the role stripe at a box's left edge (a lane header's top).
pub(super) const ROLE_STRIPE: f32 = 3.;
/// Where each role's color sits in `theme::label_colors()`: blue for a
/// transport, purple for an outside system, teal for stored state, orange
/// for a state owner, green for an actor, olive for a constraint.
const TRANSPORT_COLOR: usize = 0;
const EXTERNAL_COLOR: usize = 1;
const STORE_COLOR: usize = 2;
const OWNER_COLOR: usize = 3;
const ACTOR_COLOR: usize = 5;
const CONSTRAINT_COLOR: usize = 6;

/// A canvas filling its diagram that paints `strokes` behind the boxes.
pub(super) fn stroke_canvas(strokes: Vec<DiagramStroke>) -> impl IntoElement {
    stroke_painting(strokes)
        .absolute()
        .top_0()
        .left_0()
        .size_full()
}

/// A canvas painting `strokes` from its top left corner; its caller sizes it.
pub(super) fn stroke_painting(strokes: Vec<DiagramStroke>) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| paint_strokes(bounds, &strokes, window),
    )
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

/// The color standing for a role, as a second cue next to the label:
/// orange for a state owner, blue for a transport, teal for stored state,
/// purple for an outside system, olive for a constraint, green for an actor.
pub(super) fn role_color(role: NodeRole) -> Rgba {
    let colors = theme::label_colors();
    match role {
        NodeRole::Transport => colors[TRANSPORT_COLOR],
        NodeRole::External => colors[EXTERNAL_COLOR],
        NodeRole::Store => colors[STORE_COLOR],
        NodeRole::Owner => colors[OWNER_COLOR],
        NodeRole::Actor => colors[ACTOR_COLOR],
        NodeRole::Constraint => colors[CONSTRAINT_COLOR],
    }
}
