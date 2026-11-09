use std::collections::HashMap;

use serde::Deserialize;

use super::anchor_targets::AnchorTargets;
use super::diagram_change::DiagramChange;
use super::edge_style::EdgeStyle;
use super::graph_edge::GraphEdge;
use super::graph_node::GraphNode;
use super::message_style::MessageStyle;
use super::node_role::NodeRole;
use super::sequence_lane::SequenceLane;
use super::sequence_message::SequenceMessage;
use super::studio_graph::StudioGraph;
use super::studio_sequence::StudioSequence;
use super::wire_anchor::WireAnchor;

/// The most boxes a graph may have, so it stays readable in the brief's column.
const MAX_GRAPH_NODES: usize = 14;
/// The most arrows a graph may have.
const MAX_GRAPH_EDGES: usize = 24;
/// The most boundaries a graph may have.
const MAX_GRAPH_GROUPS: usize = 5;
/// The most participants a sequence may have.
const MAX_SEQUENCE_LANES: usize = 6;
/// The most messages a sequence may have.
const MAX_SEQUENCE_MESSAGES: usize = 24;

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
enum WireRole {
    Actor,
    Owner,
    Transport,
    Store,
    Constraint,
    External,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum WireEdgeStyle {
    Control,
    Data,
    Async,
    Optional,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum WireChange {
    Same,
    Added,
    Removed,
}

/// A sequence as the agent writes it; messages name lanes by id.
#[derive(Deserialize)]
pub(crate) struct WireSequence {
    pub(crate) title: String,
    lanes: Vec<WireLane>,
    messages: Vec<WireMessage>,
}

#[derive(Deserialize)]
struct WireLane {
    id: String,
    label: String,
    role: WireRole,
}

#[derive(Deserialize)]
struct WireMessage {
    from: String,
    to: String,
    label: String,
    style: WireMessageStyle,
    note: Option<String>,
    anchor: Option<WireAnchor>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum WireMessageStyle {
    Call,
    Reply,
    Async,
}

impl WireGraph {
    /// The graph with ids turned into indexes, if every id is known and
    /// unique, every label is written, the size is within bounds and every
    /// node's anchor is in the diff; the reason otherwise.
    pub(crate) fn checked(self, targets: &AnchorTargets) -> Result<StudioGraph, String> {
        // Size first: a graph too big to read is not worth checking further.
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

        // Ids to indexes; a repeated id would make edges ambiguous.
        let groups = index_ids(self.groups.iter().map(|group| group.id.as_str()), "group")?;
        let nodes = index_ids(self.nodes.iter().map(|node| node.id.as_str()), "node")?;

        // Each node with its group and anchor resolved.
        let mut checked_nodes = Vec::with_capacity(self.nodes.len());
        for node in self.nodes {
            require_label(&node.label, || format!("node “{}”", node.id))?;
            let group = match &node.group {
                Some(group) => Some(*groups.get(group).ok_or_else(|| {
                    format!("node “{}” is in an unknown group “{group}”", node.id)
                })?),
                None => None,
            };
            checked_nodes.push(GraphNode {
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

        // Each edge between known nodes, with the label that says what it carries.
        let node_index = |id: &str| {
            nodes
                .get(id)
                .copied()
                .ok_or_else(|| format!("an edge names an unknown node “{id}”"))
        };
        let mut checked_edges = Vec::with_capacity(self.edges.len());
        for edge in self.edges {
            require_label(&edge.label, || {
                format!("the edge from “{}” to “{}”", edge.from, edge.to)
            })?;
            let from = node_index(&edge.from)?;
            let to = node_index(&edge.to)?;
            let change = DiagramChange::from(edge.change);
            // An arrow shows only in the pictures both its boxes show in.
            let joins = [checked_nodes[from].change, checked_nodes[to].change];
            if (change.is_before() && !joins.iter().all(|node| node.is_before()))
                || (change.is_after() && !joins.iter().all(|node| node.is_after()))
            {
                return Err(format!(
                    "the edge “{}” shows in a picture (before or after) that one of its nodes is missing from",
                    edge.label.trim()
                ));
            }
            checked_edges.push(GraphEdge {
                from,
                to,
                label: edge.label.trim().to_owned(),
                style: edge.style.into(),
                change,
            });
        }

        Ok(StudioGraph {
            title: self.title,
            groups: self.groups.into_iter().map(|group| group.label).collect(),
            nodes: checked_nodes,
            edges: checked_edges,
        })
    }
}

impl WireSequence {
    /// The sequence with lane ids turned into indexes, if every id is known
    /// and unique, every label is written, the size is within bounds and
    /// every anchor is in the diff; the reason otherwise.
    pub(crate) fn checked(self, targets: &AnchorTargets) -> Result<StudioSequence, String> {
        if !(2..=MAX_SEQUENCE_LANES).contains(&self.lanes.len()) {
            return Err(format!(
                "a sequence needs 2 to {MAX_SEQUENCE_LANES} lanes, not {}",
                self.lanes.len()
            ));
        }
        if !(1..=MAX_SEQUENCE_MESSAGES).contains(&self.messages.len()) {
            return Err(format!(
                "a sequence needs 1 to {MAX_SEQUENCE_MESSAGES} messages, not {}",
                self.messages.len()
            ));
        }
        let lanes = index_ids(self.lanes.iter().map(|lane| lane.id.as_str()), "lane")?;
        let lane_index = |id: &str| {
            lanes
                .get(id)
                .copied()
                .ok_or_else(|| format!("a message names an unknown lane “{id}”"))
        };

        // Each message between known lanes, with its anchor in the diff.
        let mut messages = Vec::with_capacity(self.messages.len());
        for message in self.messages {
            require_label(&message.label, || {
                format!("the message from “{}” to “{}”", message.from, message.to)
            })?;
            messages.push(SequenceMessage {
                from: lane_index(&message.from)?,
                to: lane_index(&message.to)?,
                label: message.label.trim().to_owned(),
                style: message.style.into(),
                note: message
                    .note
                    .map(|note| note.trim().to_owned())
                    .filter(|note| !note.is_empty()),
                anchor: message
                    .anchor
                    .map(|anchor| anchor.checked(targets))
                    .transpose()?,
            });
        }

        Ok(StudioSequence {
            title: self.title,
            lanes: self
                .lanes
                .into_iter()
                .map(|lane| SequenceLane {
                    label: lane.label.trim().to_owned(),
                    role: lane.role.into(),
                })
                .collect(),
            messages,
        })
    }
}

/// Each id with its position; an error naming the first repeated one.
fn index_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    kind: &str,
) -> Result<HashMap<String, usize>, String> {
    let mut indexes = HashMap::new();
    for (index, id) in ids.enumerate() {
        if indexes.insert(id.to_owned(), index).is_some() {
            return Err(format!("the {kind} id “{id}” is used twice"));
        }
    }
    Ok(indexes)
}

/// An error naming what lacks a label, when `label` is blank: an unnamed
/// box or arrow explains nothing.
fn require_label(label: &str, what: impl FnOnce() -> String) -> Result<(), String> {
    if label.trim().is_empty() {
        return Err(format!("{} has no label", what()));
    }
    Ok(())
}

impl From<WireRole> for NodeRole {
    fn from(role: WireRole) -> Self {
        match role {
            WireRole::Actor => Self::Actor,
            WireRole::Owner => Self::Owner,
            WireRole::Transport => Self::Transport,
            WireRole::Store => Self::Store,
            WireRole::Constraint => Self::Constraint,
            WireRole::External => Self::External,
        }
    }
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

impl From<WireChange> for DiagramChange {
    fn from(change: WireChange) -> Self {
        match change {
            WireChange::Same => Self::Same,
            WireChange::Added => Self::Added,
            WireChange::Removed => Self::Removed,
        }
    }
}

impl From<WireMessageStyle> for MessageStyle {
    fn from(style: WireMessageStyle) -> Self {
        match style {
            WireMessageStyle::Call => Self::Call,
            WireMessageStyle::Reply => Self::Reply,
            WireMessageStyle::Async => Self::Async,
        }
    }
}
