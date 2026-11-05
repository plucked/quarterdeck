# qc-file-tree

| Module | Holds |
|---|---|
| `file_tree` | `FileTree::build(paths)`: folders first, name order ignoring case, single-folder chains compacted; `file_order()`, `files_under(node)`, `visible_rows(collapsed folder paths, is_shown)` |
| `tree_node` / `node_id` / `tree_row` | `TreeNode` (`name` as shown, full `path`), `TreeNodeKind`, `NodeId`, `TreeRow` |
| `file_filter` | `FileFilter`: hide viewed / tests / generated / extensions |
| `file_category` | path heuristics: `is_test_path`, `is_generated_path` (lock files, minified, `.pb.go`, `.designer.cs`, Unity `.meta`, generated folders), `extension_of` |
| `extension_count` | `ExtensionCount::of(paths)`, most files first |

- UI-free and dependency-free. File indexes are positions in the paths the tree was built from; the app builds the tree, reorders its files by `file_order()`, and builds again so indexes equal display order.
- Test/generated detection is heuristic (GitHub's API has no linguist data); extend the lists in `file_category.rs` with a test case for each new pattern.
