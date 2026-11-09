use super::diagram_change::DiagramChange;
use super::edge_style::EdgeStyle;

/// One arrow of a graph, between node indexes; its label names what it
/// carries or why the transition happens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub style: EdgeStyle,
    pub change: DiagramChange,
}
