# qc-file-tree

| Module | Holds |
|---|---|
| `file_tree` | `FileTree::build(paths)`: folders first, name order ignoring case, single-folder chains compacted; `file_order()`, `files_under(node)`, `visible_rows(collapsed folder paths, is_shown)` |
| `tree_node` / `node_id` / `tree_row` | `TreeNode` (`name` as shown, full `path`), `TreeNodeKind`, `NodeId`, `TreeRow` |
| `file_filter` / `file_group` / `filtered_file` | `FileFilter`: hidden `FileGroup`s (viewed, tests, generated, unchanged moves) and file types; `hide_all` / `show_all`; `FilteredFile` is what it looks at (path, viewed, unchanged move) |
| `file_category` | path heuristics: `is_test_path`, `is_generated_path` (lock files, minified, `.pb.go`, `.designer.cs`, Unity `.meta`, generated folders), `extension_of`, `file_type_of` (the extension, else the file name: `Dockerfile`, `.gitignore`) |
| `file_type_count` | `FileTypeCount::of(paths)`, most files first; every file has a type |

- UI-free and dependency-free. File indexes are positions in the paths the tree was built from; the app builds the tree, reorders its files by `file_order()`, and builds again so indexes equal display order.
- Test/generated detection is heuristic (GitHub's API has no linguist data); extend the lists in `file_category.rs` with a test case for each new pattern.
