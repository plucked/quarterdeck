use super::code_anchor::CodeAnchor;
use super::diagram_change::DiagramChange;
use super::node_role::NodeRole;

/// One box of a graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    /// The name in the box, such as `World thread`.
    pub label: String,
    /// A short line under the name (what it does or holds); may be empty.
    pub detail: String,
    pub role: NodeRole,
    /// The index of the boundary (thread, process) it sits in, if any.
    pub group: Option<usize>,
    /// The lines that implement it; clicking the box opens them.
    pub anchor: Option<CodeAnchor>,
    pub change: DiagramChange,
}
