use super::node_role::NodeRole;

/// One participant of a sequence: a column with its lifeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceLane {
    pub label: String,
    pub role: NodeRole,
}
