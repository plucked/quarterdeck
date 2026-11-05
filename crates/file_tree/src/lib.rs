//! A pull request's changed files as a folder tree: folders before files,
//! single-child folders compacted, one order shared with the diff, and the
//! filters that hide viewed files, tests, generated files or extensions.
//! UI-free; the app's file tree panel renders it.
//!
//! Files:
//! - `file_tree.rs`: `FileTree`, building, ordering and the visible rows.
//! - `tree_node.rs`: `TreeNode` and `TreeNodeKind`, a folder or a file.
//! - `node_id.rs`: `NodeId`, a node's index in its tree.
//! - `tree_row.rs`: `TreeRow`, one visible row with its depth.
//! - `file_filter.rs`: `FileFilter`, which files to hide.
//! - `file_category.rs`: whether a path is a test or generated, and its extension.
//! - `extension_count.rs`: `ExtensionCount`, how many files have each extension.

mod extension_count;
mod file_category;
mod file_filter;
mod file_tree;
mod node_id;
mod tree_node;
mod tree_row;

pub use extension_count::ExtensionCount;
pub use file_category::{extension_of, is_generated_path, is_test_path};
pub use file_filter::FileFilter;
pub use file_tree::FileTree;
pub use node_id::NodeId;
pub use tree_node::{TreeNode, TreeNodeKind};
pub use tree_row::TreeRow;
