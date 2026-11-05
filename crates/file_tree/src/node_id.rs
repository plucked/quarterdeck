/// A node of a [`crate::FileTree`]: an index into its nodes, valid for that tree only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub(crate) usize);
