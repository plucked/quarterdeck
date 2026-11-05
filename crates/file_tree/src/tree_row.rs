use crate::NodeId;

/// One visible row of the tree: a node and how deep it is (0 at the top).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeRow {
    pub node: NodeId,
    pub depth: usize,
}
