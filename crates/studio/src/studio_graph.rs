use super::diagram_change::DiagramChange;
use super::graph_edge::GraphEdge;
use super::graph_node::GraphNode;

/// A box-and-arrow diagram: who owns what, where data and control go,
/// which states lead where. The agent names boxes, boundaries and arrows;
/// the app lays them out. Edges and groups refer to nodes and groups by
/// index, checked when the studio is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudioGraph {
    pub title: String,
    /// The boundaries (thread, process, crate) boxes can sit in, by label.
    pub groups: Vec<String>,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl StudioGraph {
    /// Whether the change added or removed parts, so the graph is drawn
    /// before and after.
    pub fn has_changes(&self) -> bool {
        self.nodes
            .iter()
            .map(|node| node.change)
            .chain(self.edges.iter().map(|edge| edge.change))
            .any(|change| change != DiagramChange::Same)
    }
}
