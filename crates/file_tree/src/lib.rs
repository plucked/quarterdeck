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
//! - `filtered_file.rs`: `FilteredFile`, what the filter looks at in a file.
//! - `file_group.rs`: `FileGroup`, a group of files the filter hides as a whole.
//! - `file_category.rs`: whether a path is a test or generated, and its extension.
//! - `file_type_count.rs`: `FileTypeCount`, how many files have each file type (extension, or the name of a file without one).

mod file_category;
mod file_filter;
mod file_group;
mod file_tree;
mod file_type_count;
mod filtered_file;
mod node_id;
mod tree_node;
mod tree_row;

pub use file_category::{extension_of, file_type_of, is_generated_path, is_test_path};
pub use file_filter::FileFilter;
pub use file_group::FileGroup;
pub use file_tree::FileTree;
pub use file_type_count::FileTypeCount;
pub use filtered_file::FilteredFile;
pub use node_id::NodeId;
pub use tree_node::{TreeNode, TreeNodeKind};
pub use tree_row::TreeRow;
