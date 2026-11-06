# qd-file-tree

| Module | Holds |
|---|---|
| `file_tree` | `FileTree::build(paths)`: folders first, name order ignoring case, single-folder chains compacted; `file_order()`, `files_under(node)`, `folder_paths()` (every folder, for collapse all), `visible_rows(collapsed folder paths, is_shown)` |
| `tree_node` / `node_id` / `tree_row` | `TreeNode` (`name` as shown, full `path`), `TreeNodeKind`, `NodeId`, `TreeRow` |
| `file_filter` / `filtered_file` | `FileFilter`: `hide_viewed` switch + two chip rows (`kinds`, `file_types`) that must both admit a file; faceted counts (`kind_counts`, `file_type_counts` (alphabetical, ignoring the leading dot and case): each chip's total and what the other row lets through) and `describe()`; `FilteredFile` is what it looks at (the file's `FileFacets`, viewed) |
| `file_facets` | `FileFacets`: a file's kind and type, worked out once from its path (the app keeps them per loaded file; path analysis per refresh was 0.3 s for 17k files) |
| `file_kind` | `FileKind` (Code, Tests, Generated, Moved): one per file, Moved > Generated > Tests > Code |
| `facet_filter` / `facet_choice` / `facet_count` | `FacetFilter<T>`: picked (`only`) and `hidden` values of one row; nothing picked admits all; `admits` for one value per item, `is_set`; `admits_any` for several (a pull request's labels; any picked one admits, any hidden one refuses; the app's Others filters reuse it); `FacetChoice` (Open, Only, Hidden); `FacetCount` |
| `file_category` | path heuristics: `is_test_path`, `is_generated_path` (lock files, minified, `.pb.go`, `.designer.cs`, Unity `.meta`, generated folders), `extension_of`, `file_type_of` (the extension, else the file name: `Dockerfile`, `.gitignore`) |

- UI-free and dependency-free. File indexes are positions in the paths the tree was built from; the app builds the tree, reorders its files by `file_order()`, and builds again so indexes equal display order.
- Test/generated detection is heuristic (GitHub's API has no linguist data); extend the lists in `file_category.rs` with a test case for each new pattern.
