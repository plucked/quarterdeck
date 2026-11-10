use std::collections::HashMap;

use serde::Deserialize;

use super::anchor_targets::AnchorTargets;
use super::diagram_change::DiagramChange;
use super::diagram_checks::{index_ids, require_label};
use super::edge_style::EdgeStyle;
use super::graph_edge::GraphEdge;
use super::graph_node::GraphNode;
use super::studio_graph::StudioGraph;
use super::wire_anchor::WireAnchor;
use super::wire_role::WireRole;

/// The most boxes a graph may have, so it stays readable in the brief's column.
const MAX_GRAPH_NODES: usize = 14;
/// The most arrows a graph may have.
const MAX_GRAPH_EDGES: usize = 24;
/// The most boundaries a graph may have.
const MAX_GRAPH_GROUPS: usize = 5;

/// A graph as the agent writes it: nodes, groups and edges refer to each
/// other by the agent's ids.
#[derive(Deserialize)]
pub(crate) struct WireGraph {
    pub(crate) title: String,
    groups: Vec<WireGroup>,
    nodes: Vec<WireNode>,
    edges: Vec<WireEdge>,
}

#[derive(Deserialize)]
struct WireGroup {
    id: String,
    label: String,
}

#[derive(Deserialize)]
struct WireNode {
    id: String,
    label: String,
    detail: String,
    role: WireRole,
    group: Option<String>,
    anchor: Option<WireAnchor>,
    change: WireChange,
}

#[derive(Deserialize)]
struct WireEdge {
    from: String,
    to: String,
    label: String,
    style: WireEdgeStyle,
    change: WireChange,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum WireEdgeStyle {
    Control,
    Data,
    Async,
    Optional,
}

impl From<WireEdgeStyle> for EdgeStyle {
    fn from(style: WireEdgeStyle) -> Self {
        match style {
            WireEdgeStyle::Control => Self::Control,
            WireEdgeStyle::Data => Self::Data,
            WireEdgeStyle::Async => Self::Async,
            WireEdgeStyle::Optional => Self::Optional,
        }
    }
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum WireChange {
    Same,
    Added,
    Removed,
}

impl From<WireChange> for DiagramChange {
    fn from(change: WireChange) -> Self {
        match change {
            WireChange::Same => Self::Same,
            WireChange::Added => Self::Added,
            WireChange::Removed => Self::Removed,
        }
    }
}

impl WireGraph {
    /// The graph with ids turned into indexes, if every id is known and
    /// unique, every label is written, the size is within bounds and every
    /// node's anchor is in the diff; the reason otherwise.
    pub(crate) fn checked(self, targets: &AnchorTargets) -> Result<StudioGraph, String> {
        // Size first: a graph too big to read is not worth checking further.
        self.check_size()?;

        // Ids to indexes; a repeated id would make edges ambiguous.
        let groups = index_ids(self.groups.iter().map(|group| group.id.as_str()), "group")?;
        let node_indexes = index_ids(self.nodes.iter().map(|node| node.id.as_str()), "node")?;

        // The boxes, then the arrows between them.
        let nodes = checked_nodes(self.nodes, &groups, targets)?;
        let edges = checked_edges(self.edges, &node_indexes, &nodes)?;
        Ok(StudioGraph {
            title: self.title,
            groups: self.groups.into_iter().map(|group| group.label).collect(),
            nodes,
            edges,
        })
    }

    /// An error when the graph has too few or too many nodes, or too many
    /// edges or groups.
    fn check_size(&self) -> Result<(), String> {
        if !(2..=MAX_GRAPH_NODES).contains(&self.nodes.len()) {
            return Err(format!(
                "a graph needs 2 to {MAX_GRAPH_NODES} nodes, not {}",
                self.nodes.len()
            ));
        }
        if self.edges.len() > MAX_GRAPH_EDGES {
            return Err(format!(
                "a graph has at most {MAX_GRAPH_EDGES} edges, not {}",
                self.edges.len()
            ));
        }
        if self.groups.len() > MAX_GRAPH_GROUPS {
            return Err(format!(
                "a graph has at most {MAX_GRAPH_GROUPS} groups, not {}",
                self.groups.len()
            ));
        }
        Ok(())
    }
}

/// Each node labelled, with its group (as the group's index) and its anchor
/// resolved.
fn checked_nodes(
    nodes: Vec<WireNode>,
    groups: &HashMap<String, usize>,
    targets: &AnchorTargets,
) -> Result<Vec<GraphNode>, String> {
    let mut checked = Vec::with_capacity(nodes.len());
    for node in nodes {
        require_label(&node.label, || format!("node “{}”", node.id))?;
        let group =
            node.group
                .as_deref()
                .map(|group| {
                    groups.get(group).copied().ok_or_else(|| {
                        format!("node “{}” is in an unknown group “{group}”", node.id)
                    })
                })
                .transpose()?;
        checked.push(GraphNode {
            label: node.label.trim().to_owned(),
            detail: node.detail.trim().to_owned(),
            role: node.role.into(),
            group,
            anchor: node
                .anchor
                .map(|anchor| anchor.checked(targets))
                .transpose()?,
            change: node.change.into(),
        });
    }
    Ok(checked)
}

/// Each edge between known nodes (`nodes`, already checked, at the indexes
/// `node_indexes` gives), with the label that says what it carries.
fn checked_edges(
    edges: Vec<WireEdge>,
    node_indexes: &HashMap<String, usize>,
    nodes: &[GraphNode],
) -> Result<Vec<GraphEdge>, String> {
    let node_index = |id: &str| {
        node_indexes
            .get(id)
            .copied()
            .ok_or_else(|| format!("an edge names an unknown node “{id}”"))
    };
    let mut checked = Vec::with_capacity(edges.len());
    for edge in edges {
        require_label(&edge.label, || {
            format!("the edge from “{}” to “{}”", edge.from, edge.to)
        })?;
        let from = node_index(&edge.from)?;
        let to = node_index(&edge.to)?;
        let change = DiagramChange::from(edge.change);
        // An arrow shows only in the pictures both its boxes show in.
        let joins = [nodes[from].change, nodes[to].change];
        if (change.is_before() && !joins.iter().all(|node| node.is_before()))
            || (change.is_after() && !joins.iter().all(|node| node.is_after()))
        {
            return Err(format!(
                "the edge “{}” shows in a picture (before or after) that one of its nodes is missing from",
                edge.label.trim()
            ));
        }
        checked.push(GraphEdge {
            from,
            to,
            label: edge.label.trim().to_owned(),
            style: edge.style.into(),
            change,
        });
    }
    Ok(checked)
}
