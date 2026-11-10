use std::cmp::Ordering;

use qd_studio::{GraphEdge, StudioGraph};

use super::diagram_geometry::{CubicCurve, DiagramPoint, DiagramRect, DiagramSize};
use super::graph_direction::GraphDirection;
use super::text_estimate::SMALL_TEXT;
use crate::count_as_f32::count_as_f32;

/// The widest an arrow's label gets before it wraps.
const EDGE_LABEL_MAX_WIDTH: f32 = 112.;
/// Space between an arrow label's text and its background's edge.
pub const EDGE_LABEL_PADDING: f32 = 4.;
/// How far past the boxes a backward arrow bends.
const BACKWARD_DROP: f32 = 30.;
/// How much further each further backward arrow bends, so they stay apart.
const BACKWARD_STEP: f32 = 22.;
/// How far right of its box a self arrow loops.
const SELF_LOOP_REACH: f32 = 44.;
/// How far an arrow within one layer bulges past its boxes.
const SAME_LAYER_REACH: f32 = 48.;
/// Space kept between two arrow labels pushed apart.
const LABEL_SPACING: f32 = 3.;
/// How far a forward arrow's control points reach into the gap between
/// layers, as a share of the gap: how straight the arrow leaves and arrives.
const FORWARD_REACH_SHARE: f32 = 0.6;
/// Where a self arrow leaves its box's right side, as a share of its height.
const SELF_LOOP_START_SHARE: f32 = 0.3;
/// Where a self arrow comes back into its box's right side, as a share of its height.
const SELF_LOOP_END_SHARE: f32 = 0.7;
/// How far right of its box a self arrow's label starts, as a share of the
/// loop's reach: inside the loop's widest point, so the label meets the line.
const SELF_LOOP_LABEL_SHARE: f32 = 0.8;

/// An arrow's path and where its label sits.
#[derive(Debug, Clone, Copy)]
pub struct EdgeRoute {
    pub curve: CubicCurve,
    pub label: DiagramRect,
}

/// How an edge runs, given the layers of its ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeCourse {
    /// To a later layer: from the box's far side to the next box's near side.
    Forward,
    /// Within one layer: out of the far side, bulging, back into it.
    SameLayer,
    /// To an earlier layer: around the boxes between, on the far side across.
    Backward,
    /// From a box to itself: a loop on its right.
    SelfLoop,
}

/// Which side of a box a forward arrow meets it on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoxSide {
    /// The far side in the arrows' direction, where arrows leave.
    Outgoing,
    /// The near side, where arrows arrive.
    Incoming,
}

impl BoxSide {
    /// The node of `edge` on this side.
    fn end_of(self, edge: &GraphEdge) -> usize {
        match self {
            Self::Outgoing => edge.from,
            Self::Incoming => edge.to,
        }
    }

    /// The node at `edge`'s other end.
    fn other_end_of(self, edge: &GraphEdge) -> usize {
        match self {
            Self::Outgoing => edge.to,
            Self::Incoming => edge.from,
        }
    }
}

/// What routing the arrows needs to know about the placed boxes.
#[derive(Clone, Copy)]
pub(super) struct Placement<'a> {
    pub(super) graph: &'a StudioGraph,
    pub(super) direction: GraphDirection,
    pub(super) nodes: &'a [DiagramRect],
    pub(super) layers: &'a [usize],
}

/// Each edge's path and label place, by the course its layers give it.
pub(super) fn route_edges(placement: Placement) -> Vec<EdgeRoute> {
    let Placement {
        graph,
        direction,
        nodes,
        layers,
    } = placement;
    // A cycle-closing edge always ends in an earlier layer than it starts.
    let course_of = |edge: &GraphEdge| {
        if edge.from == edge.to {
            return EdgeCourse::SelfLoop;
        }
        match layers[edge.to].cmp(&layers[edge.from]) {
            Ordering::Greater => EdgeCourse::Forward,
            Ordering::Equal => EdgeCourse::SameLayer,
            Ordering::Less => EdgeCourse::Backward,
        }
    };
    let forward: Vec<usize> = (0..graph.edges.len())
        .filter(|&index| course_of(&graph.edges[index]) == EdgeCourse::Forward)
        .collect();
    let mut backward_count = 0;
    graph
        .edges
        .iter()
        .enumerate()
        .map(|(index, edge)| {
            let label_size = edge_label_size(&edge.label);
            match course_of(edge) {
                EdgeCourse::Forward => forward_route(placement, &forward, index, label_size),
                EdgeCourse::SameLayer => {
                    same_layer_route(direction, nodes[edge.from], nodes[edge.to], label_size)
                }
                EdgeCourse::Backward => {
                    // Past every box of the layers it spans; each further
                    // backward arrow bends further, so they stay apart.
                    let (first_layer, last_layer) = (layers[edge.to], layers[edge.from]);
                    let furthest = nodes
                        .iter()
                        .zip(layers)
                        .filter(|(_, layer)| (first_layer..=last_layer).contains(*layer))
                        .map(|(rect, _)| direction.across_span(*rect).1)
                        .fold(f32::MIN, f32::max);
                    let bend =
                        furthest + BACKWARD_DROP + BACKWARD_STEP * count_as_f32(backward_count);
                    backward_count += 1;
                    backward_route(
                        direction,
                        nodes[edge.from],
                        nodes[edge.to],
                        bend,
                        label_size,
                    )
                }
                EdgeCourse::SelfLoop => self_loop_route(nodes[edge.from], label_size),
            }
        })
        .collect()
}

/// A forward arrow: from its start box's far side to its end box's near
/// side, at places spreading the arrows that share a side in the order of
/// their other ends; the label in the gap after the start.
fn forward_route(
    placement: Placement,
    forward: &[usize],
    index: usize,
    label_size: DiagramSize,
) -> EdgeRoute {
    let Placement {
        graph,
        direction,
        nodes,
        ..
    } = placement;
    let centre_across = |rect: DiagramRect| {
        let (start, end) = direction.across_span(rect);
        f32::midpoint(start, end)
    };
    // Where on a side the arrow meets the box, among the forward arrows sharing that side.
    let port = |node: usize, side: BoxSide| {
        let mut sharing: Vec<usize> = forward
            .iter()
            .copied()
            .filter(|&other| side.end_of(&graph.edges[other]) == node)
            .collect();
        let other_end = |edge: usize| centre_across(nodes[side.other_end_of(&graph.edges[edge])]);
        sharing.sort_by(|left, right| other_end(*left).total_cmp(&other_end(*right)));
        let position = sharing
            .iter()
            .position(|other| *other == index)
            .unwrap_or(0);
        let (start, end) = direction.across_span(nodes[node]);
        start + (end - start) * count_as_f32(position + 1) / count_as_f32(sharing.len() + 1)
    };
    let edge = &graph.edges[index];
    let start_along = direction.along_span(nodes[edge.from]).1;
    let end_along = direction.along_span(nodes[edge.to]).0;
    let start_across = port(edge.from, BoxSide::Outgoing);
    let end_across = port(edge.to, BoxSide::Incoming);
    let reach = direction.layer_gap() * FORWARD_REACH_SHARE;
    let curve = CubicCurve {
        start: direction.point(start_along, start_across),
        control_start: direction.point(start_along + reach, start_across),
        control_end: direction.point(end_along - reach, end_across),
        end: direction.point(end_along, end_across),
    };
    let middle = curve.point_where(
        |point| direction.along_of(point),
        start_along + direction.layer_gap() / 2.,
    );
    EdgeRoute {
        curve,
        label: centred(middle, label_size),
    }
}

/// An arrow within one layer: out of the start box's far side, bulging
/// past both boxes, back into the end box's far side; the label beyond
/// the bulge.
fn same_layer_route(
    direction: GraphDirection,
    from: DiagramRect,
    to: DiagramRect,
    label_size: DiagramSize,
) -> EdgeRoute {
    let centre = |rect: DiagramRect| {
        let (start, end) = direction.across_span(rect);
        f32::midpoint(start, end)
    };
    let (from_end, to_end) = (direction.along_span(from).1, direction.along_span(to).1);
    let bulge = from_end.max(to_end) + SAME_LAYER_REACH;
    let curve = CubicCurve {
        start: direction.point(from_end, centre(from)),
        control_start: direction.point(bulge, centre(from)),
        control_end: direction.point(bulge, centre(to)),
        end: direction.point(to_end, centre(to)),
    };
    let middle = curve.point_at(0.5);
    let label_along = direction.along_and_across(label_size).0;
    let beyond = direction.point(
        direction.along_of(middle) + EDGE_LABEL_PADDING + label_along / 2.,
        direction.across_of(middle),
    );
    EdgeRoute {
        curve,
        label: centred(beyond, label_size),
    }
}

/// An arrow to an earlier layer: out of the start box's far side across
/// the arrows, around to `bend` past the boxes between, into the end box's
/// same side; the label at the bend.
fn backward_route(
    direction: GraphDirection,
    from: DiagramRect,
    to: DiagramRect,
    bend: f32,
    label_size: DiagramSize,
) -> EdgeRoute {
    let side = |rect: DiagramRect| {
        let (start, end) = direction.along_span(rect);
        (f32::midpoint(start, end), direction.across_span(rect).1)
    };
    let ((from_along, from_across), (to_along, to_across)) = (side(from), side(to));
    let curve = CubicCurve {
        start: direction.point(from_along, from_across),
        control_start: direction.point(from_along, bend),
        control_end: direction.point(to_along, bend),
        end: direction.point(to_along, to_across),
    };
    EdgeRoute {
        curve,
        label: centred(curve.point_at(0.5), label_size),
    }
}

/// An arrow from a box to itself: a loop on its right, the label beside it.
fn self_loop_route(node: DiagramRect, label_size: DiagramSize) -> EdgeRoute {
    let start = DiagramPoint {
        x: node.right(),
        y: node.y + node.height * SELF_LOOP_START_SHARE,
    };
    let end = DiagramPoint {
        x: node.right(),
        y: node.y + node.height * SELF_LOOP_END_SHARE,
    };
    let curve = CubicCurve {
        start,
        control_start: DiagramPoint {
            x: start.x + SELF_LOOP_REACH,
            y: start.y - SELF_LOOP_REACH / 2.,
        },
        control_end: DiagramPoint {
            x: end.x + SELF_LOOP_REACH,
            y: end.y + SELF_LOOP_REACH / 2.,
        },
        end,
    };
    EdgeRoute {
        curve,
        label: DiagramRect {
            x: node.right() + SELF_LOOP_REACH * SELF_LOOP_LABEL_SHARE,
            y: node.center_y() - label_size.height / 2.,
            width: label_size.width,
            height: label_size.height,
        },
    }
}

/// An arrow label's width and height: its text wrapped to the widest a
/// label gets, with padding.
fn edge_label_size(label: &str) -> DiagramSize {
    let width = SMALL_TEXT.width_of(label).min(EDGE_LABEL_MAX_WIDTH);
    DiagramSize {
        width: width + 2. * EDGE_LABEL_PADDING,
        height: SMALL_TEXT.height_of(label, EDGE_LABEL_MAX_WIDTH) + EDGE_LABEL_PADDING,
    }
}

/// A rectangle of `size` centred on `point`.
fn centred(point: DiagramPoint, size: DiagramSize) -> DiagramRect {
    DiagramRect {
        x: point.x - size.width / 2.,
        y: point.y - size.height / 2.,
        width: size.width,
        height: size.height,
    }
}

/// Moves labels down, top to bottom, until none covers another.
pub(super) fn separate_labels(edges: &mut [EdgeRoute]) {
    let mut order: Vec<usize> = (0..edges.len()).collect();
    order.sort_by(|left, right| edges[*left].label.y.total_cmp(&edges[*right].label.y));
    let mut placed: Vec<DiagramRect> = Vec::with_capacity(edges.len());
    for index in order {
        let mut label = edges[index].label;
        while let Some(covered) = placed.iter().find(|other| other.overlaps(&label)) {
            label.y = covered.bottom() + LABEL_SPACING;
        }
        edges[index].label = label;
        placed.push(label);
    }
}
