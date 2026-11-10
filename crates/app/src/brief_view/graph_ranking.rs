use std::collections::HashMap;

use qd_studio::StudioGraph;

use super::graph_bands::GraphBands;
use super::graph_cell::GraphCell;
use crate::count_as_f32::count_as_f32;

/// Which edges close a cycle (and self loops), by edge index: a
/// depth-first walk in node order marks each edge into a node still on
/// the walk's path. Leaving them out makes the rest acyclic.
pub(super) fn cycle_closing_edges(graph: &StudioGraph) -> Vec<bool> {
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
pub(super) fn node_layers(graph: &StudioGraph, backward: &[bool]) -> Vec<usize> {
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
pub(super) fn node_bands(graph: &StudioGraph) -> GraphBands {
    let mut band_groups: Vec<Option<usize>> = Vec::new();
    let of_node = graph
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
    GraphBands {
        of_node,
        groups: band_groups,
    }
}

/// The nodes of each (band, layer) cell, in drawing order: layer by layer,
/// each box after the average place of the boxes its arrows come from (so
/// arrows cross less), else in the order given.
pub(super) fn ordered_cells(
    graph: &StudioGraph,
    layers: &[usize],
    bands: &[usize],
    backward: &[bool],
) -> Vec<GraphCell> {
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
        let (band, _) = *key;
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
                let fallback = count_as_f32(band)
                    + count_as_f32(position + 1) / count_as_f32(members.len() + 1);
                (source_place(node).unwrap_or(fallback), node)
            })
            .collect();
        keyed.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)));
        *members = keyed.into_iter().map(|(_, node)| node).collect();
        // A node's place: its band, then its position in the cell, as one number.
        for (position, &node) in members.iter().enumerate() {
            place[node] =
                count_as_f32(band) + count_as_f32(position + 1) / count_as_f32(members.len() + 1);
        }
    }
    keys.into_iter()
        .map(|(band, layer)| GraphCell {
            band,
            layer,
            nodes: cells.remove(&(band, layer)).unwrap_or_default(),
        })
        .collect()
}
