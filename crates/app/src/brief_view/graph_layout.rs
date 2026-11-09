use std::cmp::Ordering;
use std::collections::HashMap;

use qd_studio::{GraphEdge, GraphNode, StudioGraph};

use super::diagram_geometry::{CubicCurve, DiagramPoint, DiagramRect};
use super::text_estimate::{LABEL_TEXT, SMALL_TEXT, count_as_f32};

/// Width of every box.
pub const NODE_WIDTH: f32 = 168.;
/// Space between a box's border and its text, sideways.
pub const NODE_PADDING_X: f32 = 10.;
/// Space between a box's border and its text, above and below.
pub const NODE_PADDING_Y: f32 = 8.;
/// Space between a boundary's border and the boxes in it.
const GROUP_PADDING: f32 = 12.;
/// Height of a boundary's name above its boxes.
pub const GROUP_LABEL_HEIGHT: f32 = 22.;
/// Space between two bands (boundaries).
const BAND_GAP: f32 = 28.;
/// Space around the whole diagram.
const MARGIN: f32 = 6.;
/// The width diagrams are laid out for until the brief's column has been
/// measured (the first frame); then they are laid out for the column.
pub const DEFAULT_DIAGRAM_WIDTH: f32 = 832.;
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

/// Which way a graph's arrows run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphDirection {
    /// Layers are columns; boundaries are horizontal bands.
    LeftToRight,
    /// Layers are rows; boundaries are vertical bands.
    TopToBottom,
}

impl GraphDirection {
    /// Space between two layers, where arrows and their labels go: labels
    /// sit beside each other in a column gap, above each other in a row gap.
    fn layer_gap(self) -> f32 {
        match self {
            Self::LeftToRight => 132.,
            Self::TopToBottom => 72.,
        }
    }

    /// Space between two boxes of one layer and band.
    fn stack_gap(self) -> f32 {
        match self {
            Self::LeftToRight => 18.,
            Self::TopToBottom => 28.,
        }
    }

    /// The point at `along` (the arrows' direction) and `across`.
    fn point(self, along: f32, across: f32) -> DiagramPoint {
        match self {
            Self::LeftToRight => DiagramPoint {
                x: along,
                y: across,
            },
            Self::TopToBottom => DiagramPoint {
                x: across,
                y: along,
            },
        }
    }

    /// The rectangle at `along` and `across`, `along_size` long in the
    /// arrows' direction and `across_size` across it.
    fn rect(self, along: f32, across: f32, along_size: f32, across_size: f32) -> DiagramRect {
        let origin = self.point(along, across);
        let (width, height) = match self {
            Self::LeftToRight => (along_size, across_size),
            Self::TopToBottom => (across_size, along_size),
        };
        DiagramRect {
            x: origin.x,
            y: origin.y,
            width,
            height,
        }
    }

    /// A point's position in the arrows' direction.
    fn along_of(self, point: DiagramPoint) -> f32 {
        match self {
            Self::LeftToRight => point.x,
            Self::TopToBottom => point.y,
        }
    }

    /// A point's position across the arrows' direction.
    fn across_of(self, point: DiagramPoint) -> f32 {
        match self {
            Self::LeftToRight => point.y,
            Self::TopToBottom => point.x,
        }
    }

    /// A rectangle's start and end in the arrows' direction.
    fn along_span(self, rect: DiagramRect) -> (f32, f32) {
        match self {
            Self::LeftToRight => (rect.x, rect.right()),
            Self::TopToBottom => (rect.y, rect.bottom()),
        }
    }

    /// A rectangle's start and end across the arrows' direction.
    fn across_span(self, rect: DiagramRect) -> (f32, f32) {
        match self {
            Self::LeftToRight => (rect.y, rect.bottom()),
            Self::TopToBottom => (rect.x, rect.right()),
        }
    }

    /// A size (width, height) as its length in the arrows' direction and across it.
    fn along_and_across(self, (width, height): (f32, f32)) -> (f32, f32) {
        match self {
            Self::LeftToRight => (width, height),
            Self::TopToBottom => (height, width),
        }
    }

    /// Room a boundary needs before its boxes, across the arrows: its name
    /// sits above the boxes, which is across only for horizontal bands.
    fn band_lead(self) -> f32 {
        match self {
            Self::LeftToRight => GROUP_LABEL_HEIGHT + GROUP_PADDING,
            Self::TopToBottom => GROUP_PADDING,
        }
    }
}

/// Where every part of a graph goes: boxes, boundaries, arrows and their
/// labels. Computed once when the brief is prepared; one layout serves the
/// before and the after picture, so their parts line up.
#[derive(Debug, Clone)]
pub struct GraphLayout {
    pub width: f32,
    pub height: f32,
    /// Each node's box, by node index.
    pub nodes: Vec<DiagramRect>,
    /// Each group's border, by group index; `None` for a group no node sits in.
    pub groups: Vec<Option<DiagramRect>>,
    /// Each edge's path and label, by edge index.
    pub edges: Vec<EdgeRoute>,
}

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
struct Placement<'a> {
    graph: &'a StudioGraph,
    direction: GraphDirection,
    nodes: &'a [DiagramRect],
    layers: &'a [usize],
}

/// Lays `graph` out along its arrows, left to right when that fits
/// `width` (the brief's column), else top to bottom when that is narrower: each node one
/// layer after the furthest node with an arrow into it (arrows closing a
/// cycle run back around the boxes); each boundary as a band of its own so
/// boundaries never overlap; boxes ordered in their layer after the boxes
/// their arrows come from; arrow labels in the gap after the arrow's start,
/// pushed apart where they would overlap.
pub fn lay_out_graph(graph: &StudioGraph, width: f32) -> GraphLayout {
    let across = lay_out_graph_in(graph, GraphDirection::LeftToRight);
    if across.width <= width {
        return across;
    }
    let down = lay_out_graph_in(graph, GraphDirection::TopToBottom);
    if down.width < across.width {
        down
    } else {
        across
    }
}

/// Lays `graph` out with its arrows running in `direction`.
fn lay_out_graph_in(graph: &StudioGraph, direction: GraphDirection) -> GraphLayout {
    // Which layer and band each box goes in.
    let heights: Vec<f32> = graph.nodes.iter().map(node_height).collect();
    let backward = cycle_closing_edges(graph);
    let layers = node_layers(graph, &backward);
    let (bands, band_groups) = node_bands(graph);

    // Place the boxes, then the boundaries around them.
    let cells = ordered_cells(graph, &layers, &bands, &backward);
    let nodes = place_nodes(direction, &heights, &layers, &cells, &band_groups);
    let groups: Vec<Option<DiagramRect>> = (0..graph.groups.len())
        .map(|group| group_border(graph, &nodes, group))
        .collect();

    // Route the arrows, then keep their labels from covering each other.
    let mut edges = route_edges(Placement {
        graph,
        direction,
        nodes: &nodes,
        layers: &layers,
    });
    separate_labels(&mut edges);

    // Move everything so the furthest parts sit a margin inside the diagram.
    let extent = nodes
        .iter()
        .chain(groups.iter().flatten())
        .copied()
        .chain(
            edges
                .iter()
                .flat_map(|edge| [edge.label, edge.curve.bounds()]),
        )
        .fold(
            (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
            |(min_x, min_y, max_x, max_y), rect| {
                (
                    min_x.min(rect.x),
                    min_y.min(rect.y),
                    max_x.max(rect.right()),
                    max_y.max(rect.bottom()),
                )
            },
        );
    let (min_x, min_y, max_x, max_y) = extent;
    let (shift_x, shift_y) = (MARGIN - min_x, MARGIN - min_y);
    GraphLayout {
        width: max_x - min_x + 2. * MARGIN,
        height: max_y - min_y + 2. * MARGIN,
        nodes: nodes
            .iter()
            .map(|rect| rect.moved(shift_x, shift_y))
            .collect(),
        groups: groups
            .iter()
            .map(|rect| rect.map(|rect| rect.moved(shift_x, shift_y)))
            .collect(),
        edges: edges
            .iter()
            .map(|edge| EdgeRoute {
                curve: edge.curve.moved(shift_x, shift_y),
                label: edge.label.moved(shift_x, shift_y),
            })
            .collect(),
    }
}

/// A box's height: its name and detail wrapped to the box, with padding.
fn node_height(node: &GraphNode) -> f32 {
    let text_width = NODE_WIDTH - 2. * NODE_PADDING_X;
    let label = LABEL_TEXT.height_of(&node.label, text_width);
    if node.detail.is_empty() {
        return 2. * NODE_PADDING_Y + label;
    }
    2. * NODE_PADDING_Y + label + SMALL_TEXT.height_of(&node.detail, text_width)
}

/// Each box's rectangle: layers one after another in the arrows'
/// direction, each as long as its longest box; bands side by side across
/// it, each as wide as its widest layer (with room for a boundary's border
/// and name); a cell's boxes stacked across the arrows, centred in the band.
fn place_nodes(
    direction: GraphDirection,
    heights: &[f32],
    layers: &[usize],
    cells: &[((usize, usize), Vec<usize>)],
    band_groups: &[Option<usize>],
) -> Vec<DiagramRect> {
    let sizes: Vec<(f32, f32)> = heights
        .iter()
        .map(|height| direction.along_and_across((NODE_WIDTH, *height)))
        .collect();

    // Where each layer starts in the arrows' direction.
    let layer_count = layers.iter().max().map_or(0, |layer| layer + 1);
    let mut thickness = vec![0_f32; layer_count];
    for (node, layer) in layers.iter().enumerate() {
        thickness[*layer] = thickness[*layer].max(sizes[node].0);
    }
    let mut layer_start = Vec::with_capacity(layer_count);
    let mut along = 0.;
    for layer_thickness in &thickness {
        layer_start.push(along);
        along += layer_thickness + direction.layer_gap();
    }

    // Each band across the arrows, its cells' boxes centred in it.
    let stack_extent = |members: &[usize]| {
        members.iter().map(|&node| sizes[node].1).sum::<f32>()
            + direction.stack_gap() * count_as_f32(members.len().saturating_sub(1))
    };
    let mut nodes = vec![
        DiagramRect {
            x: 0.,
            y: 0.,
            width: 0.,
            height: 0.,
        };
        heights.len()
    ];
    let mut band_start = 0.;
    for (band, group) in band_groups.iter().enumerate() {
        let band_cells = || {
            cells
                .iter()
                .filter(move |((cell_band, _), _)| *cell_band == band)
        };
        let extent = band_cells()
            .map(|(_, members)| stack_extent(members))
            .fold(0., f32::max);
        let content_start = band_start + group.map_or(0., |_| direction.band_lead());
        for ((_, layer), members) in band_cells() {
            let mut across = content_start + (extent - stack_extent(members)) / 2.;
            for &node in members {
                let (along_size, across_size) = sizes[node];
                let along = layer_start[*layer] + (thickness[*layer] - along_size) / 2.;
                nodes[node] = direction.rect(along, across, along_size, across_size);
                across += across_size + direction.stack_gap();
            }
        }
        band_start = content_start + extent + group.map_or(0., |_| GROUP_PADDING) + BAND_GAP;
    }
    nodes
}

/// A boundary's border: around its boxes, with room for its name above
/// them; `None` when no box sits in it.
fn group_border(graph: &StudioGraph, nodes: &[DiagramRect], group: usize) -> Option<DiagramRect> {
    let mut members = graph
        .nodes
        .iter()
        .zip(nodes)
        .filter(|(node, _)| node.group == Some(group))
        .map(|(_, rect)| *rect);
    let first = members.next()?;
    let (min_x, min_y, max_x, max_y) = members.fold(
        (first.x, first.y, first.right(), first.bottom()),
        |(min_x, min_y, max_x, max_y), rect| {
            (
                min_x.min(rect.x),
                min_y.min(rect.y),
                max_x.max(rect.right()),
                max_y.max(rect.bottom()),
            )
        },
    );
    let top = min_y - GROUP_PADDING - GROUP_LABEL_HEIGHT;
    Some(DiagramRect {
        x: min_x - GROUP_PADDING,
        y: top,
        width: max_x - min_x + 2. * GROUP_PADDING,
        height: max_y + GROUP_PADDING - top,
    })
}

/// Which edges close a cycle (and self loops), by edge index: a
/// depth-first walk in node order marks each edge into a node still on
/// the walk's path. Leaving them out makes the rest acyclic.
fn cycle_closing_edges(graph: &StudioGraph) -> Vec<bool> {
    // Out-edges of each node, in the order given.
    let mut outgoing = vec![Vec::new(); graph.nodes.len()];
    for (index, edge) in graph.edges.iter().enumerate() {
        outgoing[edge.from].push(index);
    }
    let mut closing = vec![false; graph.edges.len()];
    let mut on_path = vec![false; graph.nodes.len()];
    let mut visited = vec![false; graph.nodes.len()];
    // An explicit stack of (node, next out-edge position) walks without recursion.
    for root in 0..graph.nodes.len() {
        if visited[root] {
            continue;
        }
        let mut stack = vec![(root, 0)];
        visited[root] = true;
        on_path[root] = true;
        while let Some((node, position)) = stack.last_mut() {
            let node = *node;
            let Some(&edge) = outgoing[node].get(*position) else {
                on_path[node] = false;
                stack.pop();
                continue;
            };
            *position += 1;
            let target = graph.edges[edge].to;
            if on_path[target] {
                closing[edge] = true;
            } else if !visited[target] {
                visited[target] = true;
                on_path[target] = true;
                stack.push((target, 0));
            }
        }
    }
    closing
}

/// Each node's layer: one after the furthest layer of a node with a (not
/// cycle-closing) edge into it, so arrows run one way.
fn node_layers(graph: &StudioGraph, backward: &[bool]) -> Vec<usize> {
    let count = graph.nodes.len();
    let mut layers = vec![0; count];
    // Relax along forward edges; the graph without them is acyclic, so
    // `count` rounds settle every layer.
    for _ in 0..count {
        let mut changed = false;
        for (edge, _) in graph
            .edges
            .iter()
            .zip(backward)
            .filter(|(_, is_backward)| !**is_backward)
        {
            if layers[edge.to] < layers[edge.from] + 1 {
                layers[edge.to] = layers[edge.from] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    layers
}

/// Each node's band, and each band's group (`None` for the band of nodes
/// in no group), in the order the bands' first nodes are listed.
fn node_bands(graph: &StudioGraph) -> (Vec<usize>, Vec<Option<usize>>) {
    let mut band_groups: Vec<Option<usize>> = Vec::new();
    let bands = graph
        .nodes
        .iter()
        .map(|node| {
            band_groups
                .iter()
                .position(|group| *group == node.group)
                .unwrap_or_else(|| {
                    band_groups.push(node.group);
                    band_groups.len() - 1
                })
        })
        .collect();
    (bands, band_groups)
}

/// The nodes of each (band, layer) cell, in drawing order: layer by layer,
/// each box after the average place of the boxes its arrows come from (so
/// arrows cross less), else in the order given.
fn ordered_cells(
    graph: &StudioGraph,
    layers: &[usize],
    bands: &[usize],
    backward: &[bool],
) -> Vec<((usize, usize), Vec<usize>)> {
    // Gather the cells.
    let mut cells: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for node in 0..graph.nodes.len() {
        cells
            .entry((bands[node], layers[node]))
            .or_default()
            .push(node);
    }
    let mut keys: Vec<(usize, usize)> = cells.keys().copied().collect();
    keys.sort_by_key(|(band, layer)| (*layer, *band));

    // Sort each cell, earlier layers first, by where its sources sit.
    let mut place = vec![0_f32; graph.nodes.len()];
    for key in &keys {
        let members = cells.get_mut(key).expect("a gathered cell");
        let source_place = |node: usize| {
            let sources: Vec<f32> = graph
                .edges
                .iter()
                .zip(backward)
                .filter(|(edge, is_backward)| !**is_backward && edge.to == node)
                .map(|(edge, _)| place[edge.from])
                .collect();
            (!sources.is_empty()).then(|| sources.iter().sum::<f32>() / count_as_f32(sources.len()))
        };
        let mut keyed: Vec<(f32, usize)> = members
            .iter()
            .enumerate()
            .map(|(position, &node)| {
                let fallback = count_as_f32(key.0)
                    + count_as_f32(position + 1) / count_as_f32(members.len() + 1);
                (source_place(node).unwrap_or(fallback), node)
            })
            .collect();
        keyed.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)));
        *members = keyed.into_iter().map(|(_, node)| node).collect();
        // A node's place: its band, then its position in the cell, as one number.
        for (position, &node) in members.iter().enumerate() {
            place[node] =
                count_as_f32(key.0) + count_as_f32(position + 1) / count_as_f32(members.len() + 1);
        }
    }
    keys.into_iter()
        .map(|key| {
            let members = cells.remove(&key).unwrap_or_default();
            (key, members)
        })
        .collect()
}

/// Each edge's path and label place, by the course its layers give it.
fn route_edges(placement: Placement) -> Vec<EdgeRoute> {
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
    label_size: (f32, f32),
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
    let reach = direction.layer_gap() * 0.6;
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
    label_size: (f32, f32),
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
    label_size: (f32, f32),
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
fn self_loop_route(node: DiagramRect, label_size: (f32, f32)) -> EdgeRoute {
    let start = DiagramPoint {
        x: node.right(),
        y: node.y + node.height * 0.3,
    };
    let end = DiagramPoint {
        x: node.right(),
        y: node.y + node.height * 0.7,
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
            x: node.right() + SELF_LOOP_REACH * 0.8,
            y: node.center_y() - label_size.1 / 2.,
            width: label_size.0,
            height: label_size.1,
        },
    }
}

/// An arrow label's width and height: its text wrapped to the widest a
/// label gets, with padding.
fn edge_label_size(label: &str) -> (f32, f32) {
    let width = SMALL_TEXT.width_of(label).min(EDGE_LABEL_MAX_WIDTH);
    (
        width + 2. * EDGE_LABEL_PADDING,
        SMALL_TEXT.height_of(label, EDGE_LABEL_MAX_WIDTH) + EDGE_LABEL_PADDING,
    )
}

/// A rectangle of `size` centred on `point`.
fn centred(point: DiagramPoint, size: (f32, f32)) -> DiagramRect {
    DiagramRect {
        x: point.x - size.0 / 2.,
        y: point.y - size.1 / 2.,
        width: size.0,
        height: size.1,
    }
}

/// Moves labels down, top to bottom, until none covers another.
fn separate_labels(edges: &mut [EdgeRoute]) {
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

#[cfg(test)]
mod tests {
    use qd_studio::{DiagramChange, EdgeStyle, NodeRole};

    use super::*;

    fn graph(node_groups: &[Option<usize>], edges: &[(usize, usize)]) -> StudioGraph {
        StudioGraph {
            title: "Test".into(),
            groups: vec!["Group".into(); 2],
            nodes: node_groups
                .iter()
                .enumerate()
                .map(|(index, group)| GraphNode {
                    label: format!("Node {index}"),
                    detail: String::new(),
                    role: NodeRole::Actor,
                    group: *group,
                    anchor: None,
                    change: DiagramChange::Same,
                })
                .collect(),
            edges: edges
                .iter()
                .map(|&(from, to)| GraphEdge {
                    from,
                    to,
                    label: "carries".into(),
                    style: EdgeStyle::Control,
                    change: DiagramChange::Same,
                })
                .collect(),
        }
    }

    /// A short chain runs left to right, one column per step; a cycle's
    /// closing edge runs back under the boxes.
    #[test]
    fn lays_a_short_chain_out_left_to_right() {
        let layout = lay_out_graph(
            &graph(&[None, None, None], &[(0, 1), (1, 2), (2, 0)]),
            DEFAULT_DIAGRAM_WIDTH,
        );
        assert!(layout.nodes[0].right() < layout.nodes[1].x);
        assert!(layout.nodes[1].right() < layout.nodes[2].x);
        assert!(layout.edges[2].curve.bounds().bottom() > layout.nodes[0].bottom());
    }

    /// A chain too long for the column runs top to bottom, so it stays narrow.
    #[test]
    fn lays_a_long_chain_out_top_to_bottom() {
        let nodes = [None; 5];
        let edges = [(0, 1), (1, 2), (2, 3), (3, 4)];
        let layout = lay_out_graph(&graph(&nodes, &edges), DEFAULT_DIAGRAM_WIDTH);
        for pair in layout.nodes.windows(2) {
            assert!(pair[0].bottom() < pair[1].y);
        }
        assert!(layout.width < 2. * NODE_WIDTH);
    }

    /// The same chain runs left to right in a wide column and top to
    /// bottom in a narrow one.
    #[test]
    fn turns_a_chain_down_when_the_column_narrows() {
        let chain = graph(&[None, None, None], &[(0, 1), (1, 2)]);
        let wide = lay_out_graph(&chain, DEFAULT_DIAGRAM_WIDTH);
        assert!(wide.nodes[0].right() < wide.nodes[1].x);
        let narrow = lay_out_graph(&chain, wide.width - 1.);
        assert!(narrow.nodes[0].bottom() < narrow.nodes[1].y);
        assert!(narrow.width < wide.width);
    }

    /// Two boundaries become bands whose borders do not overlap, each
    /// around its own boxes, in either direction.
    #[test]
    fn keeps_boundaries_apart() {
        let short = graph(&[Some(0), Some(1), Some(0)], &[(0, 1), (1, 2)]);
        let long = graph(
            &[Some(0), Some(1), Some(0), Some(1), Some(0)],
            &[(0, 1), (1, 2), (2, 3), (3, 4)],
        );
        for layout in [
            lay_out_graph(&short, DEFAULT_DIAGRAM_WIDTH),
            lay_out_graph(&long, DEFAULT_DIAGRAM_WIDTH),
        ] {
            let first = layout.groups[0].expect("group 0 has nodes");
            let second = layout.groups[1].expect("group 1 has nodes");
            assert!(!first.overlaps(&second));
            assert!(first.x <= layout.nodes[2].x && layout.nodes[2].right() <= first.right());
        }
    }

    /// Labels of arrows leaving one box never cover each other.
    #[test]
    fn keeps_labels_apart() {
        let layout = lay_out_graph(
            &graph(&[None, None, None, None], &[(0, 1), (0, 2), (0, 3)]),
            DEFAULT_DIAGRAM_WIDTH,
        );
        for (index, edge) in layout.edges.iter().enumerate() {
            for other in &layout.edges[index + 1..] {
                assert!(!edge.label.overlaps(&other.label));
            }
        }
    }
}
