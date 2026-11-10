//! A pull request's changed files as a folder tree: folders before files,
//! single-child folders compacted, one order shared with the diff, and the
//! filter: a viewed switch and chip rows by kind and by file type; and each
//! file's icon.
//! UI-free; the app's file tree panel renders it.
//!
//! Files:
//! - `file_tree.rs`: `FileTree`, building, ordering and the visible rows.
//! - `tree_node.rs`: `TreeNode` and `TreeNodeKind`, a folder or a file.
//! - `node_id.rs`: `NodeId`, a node's index in its tree.
//! - `tree_row.rs`: `TreeRow`, one visible row with its depth.
//! - `file_filter.rs`: `FileFilter`, which files show, and the chips' counts.
//! - `filtered_file.rs`: `FilteredFile`, what the filter looks at in a file.
//! - `file_facets.rs`: `FileFacets`, a file's kind and type, worked out once from its path.
//! - `file_kind.rs`: `FileKind`, a file's one kind (code, tests, generated, moved).
//! - `facet_filter.rs`: `FacetFilter`, one chip row: picked and hidden values.
//! - `facet_choice.rs`: `FacetChoice`, what one chip is set to.
//! - `facet_count.rs`: `FacetCount`, one chip's total and matching counts.
//! - `file_category.rs`: whether a path is a test or generated, and its extension.
//! - `file_icon.rs`: `FileIcon`, the icon a file shows, picked from its path.
//! - `file_icon_rules.rs`: the file names, name beginnings and extensions each icon stands for.

mod facet_choice;
mod facet_count;
mod facet_filter;
mod file_category;
mod file_facets;
mod file_filter;
mod file_icon;
mod file_icon_rules;
mod file_kind;
mod file_tree;
mod filtered_file;
mod node_id;
mod tree_node;
mod tree_row;

pub use facet_choice::FacetChoice;
pub use facet_count::FacetCount;
pub use facet_filter::FacetFilter;
pub use file_facets::FileFacets;
pub use file_filter::FileFilter;
pub use file_icon::FileIcon;
pub use file_kind::FileKind;
pub use file_tree::FileTree;
pub use filtered_file::FilteredFile;
pub use node_id::NodeId;
pub use tree_node::{TreeNode, TreeNodeKind};
pub use tree_row::TreeRow;
