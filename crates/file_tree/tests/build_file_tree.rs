//! Building and reading file trees, through the public API.

use std::collections::HashSet;

use qc_file_tree::{FileTree, TreeNodeKind};

const PATHS: [&str; 6] = [
    "README.md",
    "src/main.rs",
    "src/app/views/list.rs",
    "src/app/views/item.rs",
    "Cargo.toml",
    "docs/guide.md",
];

fn names(tree: &FileTree) -> Vec<String> {
    tree.visible_rows(&HashSet::new(), |_| true)
        .into_iter()
        .map(|row| format!("{}{}", "  ".repeat(row.depth), tree.node(row.node).name))
        .collect()
}

/// Folders come first, names sort ignoring case, and single folders compact.
#[test]
fn sorts_folders_first_and_compacts_chains() {
    let tree = FileTree::build(&PATHS);
    assert_eq!(
        names(&tree),
        [
            "docs",
            "  guide.md",
            "src",
            "  app/views",
            "    item.rs",
            "    list.rs",
            "  main.rs",
            "Cargo.toml",
            "README.md",
        ]
    );
}

/// The file order is the expanded tree read top to bottom.
#[test]
fn file_order_follows_the_tree() {
    let tree = FileTree::build(&PATHS);
    assert_eq!(tree.file_order(), [5, 3, 2, 1, 4, 0]);
}

/// A collapsed folder hides its children; a folder with nothing shown disappears.
#[test]
fn visible_rows_respect_collapsing_and_filters() {
    let tree = FileTree::build(&PATHS);
    let collapsed: HashSet<String> = ["src".to_owned()].into();
    let rows: Vec<_> = tree
        .visible_rows(&collapsed, |file| file != 5)
        .into_iter()
        .map(|row| tree.node(row.node).name.clone())
        .collect();
    assert_eq!(rows, ["src", "Cargo.toml", "README.md"]);
}

/// A folder's files are everything under it, in tree order.
#[test]
fn files_under_a_folder() {
    let tree = FileTree::build(&PATHS);
    let src = tree
        .roots()
        .iter()
        .copied()
        .find(|node| tree.node(*node).name == "src")
        .unwrap();
    assert!(matches!(tree.node(src).kind, TreeNodeKind::Folder { .. }));
    assert_eq!(tree.files_under(src), [3, 2, 1]);
    assert_eq!(tree.node(src).path, "src");
}
