use std::collections::{BTreeMap, HashSet};

use crate::{NodeId, TreeNode, TreeNodeKind, TreeRow};

/// Folders before files, each sorted by name ignoring case (then by exact
/// name), as file explorers order them.
#[derive(Default)]
struct FolderUnderConstruction {
    folders: BTreeMap<String, FolderUnderConstruction>,
    files: Vec<(String, usize)>,
}

/// A pull request's changed files as a folder tree. Folders come before
/// files and both are sorted by name; folders with a single child folder
/// and no files are compacted into one node (`src/app/views`).
///
/// [`Self::file_order`] is the order a reader meets the files going down the
/// fully expanded tree; the diff shows files in that order, so tree and diff
/// always agree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileTree {
    nodes: Vec<TreeNode>,
    roots: Vec<NodeId>,
}

impl FileTree {
    /// Builds the tree of `paths` (`/`-separated); a file node's `file` is
    /// its index in `paths`.
    pub fn build(paths: &[&str]) -> Self {
        // Gather every path into nested folders.
        let mut root = FolderUnderConstruction::default();
        for (index, path) in paths.iter().enumerate() {
            let mut segments: Vec<&str> = path
                .split('/')
                .filter(|segment| !segment.is_empty())
                .collect();
            let Some(file_name) = segments.pop() else {
                continue;
            };
            let folder = segments.iter().fold(&mut root, |folder, segment| {
                folder.folders.entry((*segment).to_owned()).or_default()
            });
            folder.files.push((file_name.to_owned(), index));
        }

        // Turn them into nodes, sorted and compacted.
        let mut tree = Self::default();
        tree.roots = tree.add_children(root, "");
        tree
    }

    /// The top-level nodes in tree order.
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn node(&self, id: NodeId) -> &TreeNode {
        &self.nodes[id.0]
    }

    /// Every file index in tree order: depth first, folders before files.
    pub fn file_order(&self) -> Vec<usize> {
        let mut order = Vec::new();
        for root in &self.roots {
            self.collect_files(*root, &mut order);
        }
        order
    }

    /// The file indexes under `node` (itself when it is a file), in tree order.
    pub fn files_under(&self, node: NodeId) -> Vec<usize> {
        let mut files = Vec::new();
        self.collect_files(node, &mut files);
        files
    }

    /// The rows to show: expanded folders show their children, folders whose
    /// path is in `collapsed` do not, and only files `is_shown` accepts
    /// appear. A folder without any shown file under it is left out.
    pub fn visible_rows(
        &self,
        collapsed: &HashSet<String>,
        is_shown: impl Fn(usize) -> bool,
    ) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        for root in &self.roots {
            self.collect_rows(*root, 0, collapsed, &is_shown, &mut rows);
        }
        rows
    }

    /// Adds the children of a folder at `parent_path` as nodes and returns their ids in order.
    fn add_children(&mut self, folder: FolderUnderConstruction, parent_path: &str) -> Vec<NodeId> {
        let mut folders: Vec<(String, FolderUnderConstruction)> =
            folder.folders.into_iter().collect();
        folders.sort_by(|left, right| compare_names(&left.0, &right.0));
        let mut files = folder.files;
        files.sort_by(|left, right| compare_names(&left.0, &right.0));

        let mut children = Vec::with_capacity(folders.len() + files.len());
        for (name, child) in folders {
            children.push(self.add_folder(name, child, parent_path));
        }
        for (name, file) in files {
            let path = join_path(parent_path, &name);
            children.push(self.push(TreeNode {
                name,
                path,
                kind: TreeNodeKind::File { file },
            }));
        }
        children
    }

    /// Adds a folder node, joining single-child folders without files into it.
    fn add_folder(
        &mut self,
        mut name: String,
        mut folder: FolderUnderConstruction,
        parent_path: &str,
    ) -> NodeId {
        // Compact `a` → `a/b` → `a/b/c` while each holds exactly one folder and no files.
        while folder.files.is_empty() && folder.folders.len() == 1 {
            let Some((child_name, child)) = folder.folders.pop_first() else {
                break;
            };
            name = format!("{name}/{child_name}");
            folder = child;
        }
        let path = join_path(parent_path, &name);
        let children = self.add_children(folder, &path);
        self.push(TreeNode {
            name,
            path,
            kind: TreeNodeKind::Folder { children },
        })
    }

    fn push(&mut self, node: TreeNode) -> NodeId {
        self.nodes.push(node);
        NodeId(self.nodes.len() - 1)
    }

    fn collect_files(&self, node: NodeId, files: &mut Vec<usize>) {
        match &self.node(node).kind {
            TreeNodeKind::File { file } => files.push(*file),
            TreeNodeKind::Folder { children } => {
                for child in children {
                    self.collect_files(*child, files);
                }
            }
        }
    }

    /// Adds `node`'s row and, for an expanded folder, its children's; returns
    /// whether anything was added (a folder with no shown file adds nothing).
    fn collect_rows(
        &self,
        node: NodeId,
        depth: usize,
        collapsed: &HashSet<String>,
        is_shown: &impl Fn(usize) -> bool,
        rows: &mut Vec<TreeRow>,
    ) -> bool {
        match &self.node(node).kind {
            TreeNodeKind::File { file } => {
                let shown = is_shown(*file);
                if shown {
                    rows.push(TreeRow { node, depth });
                }
                shown
            }
            TreeNodeKind::Folder { children } => {
                // A folder shows when something under it does, even while collapsed.
                if !self.files_under(node).into_iter().any(is_shown) {
                    return false;
                }
                rows.push(TreeRow { node, depth });
                if !collapsed.contains(&self.node(node).path) {
                    for child in children {
                        self.collect_rows(*child, depth + 1, collapsed, is_shown, rows);
                    }
                }
                true
            }
        }
    }
}

/// Name order ignoring case, then exact, so `a.rs` and `B.rs` read naturally.
fn compare_names(left: &str, right: &str) -> std::cmp::Ordering {
    left.to_lowercase()
        .cmp(&right.to_lowercase())
        .then_with(|| left.cmp(right))
}

fn join_path(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}
