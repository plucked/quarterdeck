# qc-diff

| Module | Holds |
|---|---|
| `file_diff` | `FileDiff::build(patch, DiffOptions)`, the entry point; counts and max line number |
| `diff_hunk` | `DiffHunk` + `split_rows()` for side-by-side |
| `diff_line` / `diff_line_kind` | numbered line with `changed_ranges`; context/added/removed |
| `split_row` | `SplitRow`: old/new line indexes of one side-by-side row |
| `diff_options` / `whitespace_mode` / `change_highlight` | build options |
| `parse_patch` | GitHub per-file patch → hunks (tabs → 4 spaces, `\ No newline` dropped) |
| `refine_changes` | whitespace folding (`git diff -w` semantics), then inline marks |
| `change_block` | removed run + added run; pairs by position |
| `inline_changes` | `InlineChanges::between`: word (code-aware split) or char diff via `similar`, skipped below 0.4 similarity, 5 ms timeout |

- UI-free; the app's `diff_view` renders it. Patches come from GitHub; no full-file diffing here.
- Whitespace Ignore: a removed/added pair equal without whitespace becomes one context line (new text, both numbers); hunks left without changes are dropped.
- Public-API tests in `tests/build_file_diff.rs`.
