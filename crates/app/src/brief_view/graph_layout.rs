use qd_studio::{GraphNode, StudioGraph};

use super::diagram_geometry::{DIAGRAM_MARGIN, DiagramRect, DiagramSize};
use super::edge_routing::{EdgeRoute, Placement, route_edges, separate_labels};
use super::graph_cell::GraphCell;
use super::graph_direction::GraphDirection;
use super::graph_ranking::{cycle_closing_edges, node_bands, node_layers, ordered_cells};
use super::text_estimate::{LABEL_TEXT, SMALL_TEXT};
use crate::count_as_f32::count_as_f32;

/// Width of every box.
pub const NODE_WIDTH: f32 = 168.;
/// Space between a box's border and its text, sideways.
pub const NODE_PADDING_X: f32 = 10.;
/// Space between a box's border and its text, above and below.
pub const NODE_PADDING_Y: f32 = 8.;
/// Space between a boundary's border and the boxes in it.
pub(super) const GROUP_PADDING: f32 = 12.;
/// Height of a boundary's name above its boxes.
pub const GROUP_LABEL_HEIGHT: f32 = 22.;
/// Space between two bands (boundaries).
const BAND_GAP: f32 = 28.;

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

impl GraphLayout {
    /// How much room the whole graph takes.
    pub fn size(&self) -> DiagramSize {
        DiagramSize {
            width: self.width,
            height: self.height,
        }
    }
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
    let bands = node_bands(graph);

    // Place the boxes, then the boundaries around them.
    let cells = ordered_cells(graph, &layers, &bands.of_node, &backward);
    let nodes = place_nodes(direction, &heights, &layers, &cells, &bands.groups);
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
    let extent = DiagramRect::enclosing(
        nodes.iter().chain(groups.iter().flatten()).copied().chain(
            edges
                .iter()
                .flat_map(|edge| [edge.label, edge.curve.bounds()]),
        ),
    )
    .unwrap_or_default();
    let (shift_x, shift_y) = (DIAGRAM_MARGIN - extent.x, DIAGRAM_MARGIN - extent.y);
    GraphLayout {
        width: extent.width + 2. * DIAGRAM_MARGIN,
        height: extent.height + 2. * DIAGRAM_MARGIN,
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
    cells: &[GraphCell],
    band_groups: &[Option<usize>],
) -> Vec<DiagramRect> {
    let sizes: Vec<(f32, f32)> = heights
        .iter()
        .map(|height| {
            direction.along_and_across(DiagramSize {
                width: NODE_WIDTH,
                height: *height,
            })
        })
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
    let mut nodes = vec![DiagramRect::default(); heights.len()];
    let mut band_start = 0.;
    for (band, group) in band_groups.iter().enumerate() {
        let band_cells = || cells.iter().filter(move |cell| cell.band == band);
        let extent = band_cells()
            .map(|cell| stack_extent(&cell.nodes))
            .fold(0., f32::max);
        let content_start = band_start + group.map_or(0., |_| direction.band_lead());
        for cell in band_cells() {
            let mut across = content_start + (extent - stack_extent(&cell.nodes)) / 2.;
            for &node in &cell.nodes {
                let (along_size, across_size) = sizes[node];
                let along = layer_start[cell.layer] + (thickness[cell.layer] - along_size) / 2.;
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
    let members = graph
        .nodes
        .iter()
        .zip(nodes)
        .filter(|(node, _)| node.group == Some(group))
        .map(|(_, rect)| *rect);
    let boxes = DiagramRect::enclosing(members)?;
    let top = boxes.y - GROUP_PADDING - GROUP_LABEL_HEIGHT;
    Some(DiagramRect {
        x: boxes.x - GROUP_PADDING,
        y: top,
        width: boxes.width + 2. * GROUP_PADDING,
        height: boxes.bottom() + GROUP_PADDING - top,
    })
}

#[cfg(test)]
mod tests {
    use qd_studio::{DiagramChange, EdgeStyle, GraphEdge, NodeRole};

    use super::super::diagram_geometry::DEFAULT_DIAGRAM_WIDTH;
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
