use crate::NodeId;

/// A folder or a file of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    /// What the tree shows: a file's name, or a folder's name with the
    /// single-child folders below it joined in (`src/app/views`).
    pub name: String,
    /// The full path: a file's path, or a folder's path without a trailing `/`.
    pub path: String,
    pub kind: TreeNodeKind,
}

/// Whether a node is a folder (with its children in tree order) or a file
/// (with its index in the paths the tree was built from).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNodeKind {
    Folder { children: Vec<NodeId> },
    File { file: usize },
}
