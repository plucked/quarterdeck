# qd-diff

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
| `line_origin` | `LineOrigin`: a line of the patch, or one revealed from the file's text (`DiffLine::origin`) |
| `source_lines` / `revealed_lines` / `context_gap` | `SourceLines` (the new file's text as numbered lines, tabs expanded), `RevealedLines` (merged new-side ranges shown; `within(span)` lists them clipped), `ContextGap` + `GapPlace` (hidden lines above a hunk or below the last; `top`/`bottom`/`all` parts) |
| `hunk_context` | `FileDiff::gaps(line_count)`, `FileDiff::is_patch_of(source)` (every new-side line of the hunks reads the same in the text) and `FileDiff::with_context(source, revealed)`: revealed lines join the hunks they touch (only revealed runs are walked), headers worked out again; `None` when the text is not the patch's |
| `inline_changes` | `InlineChanges::between`: word (code-aware split) or char diff via `similar`, skipped below 0.4 similarity, 5 ms timeout |

- UI-free; the app's `diff_view` renders it. Patches come from GitHub; no full-file diffing here: context lines come from the new file's text, numbered on the old side by the gap's fixed offset (nothing changes in a gap). `DiffHunk` keeps its header's `old_start`/`new_start` (for a side without lines, the line after which it sits).
- Whitespace Ignore: a removed/added pair equal without whitespace becomes one context line (new text, both numbers); hunks left without changes are dropped.
- Public-API tests in `tests/build_file_diff.rs`.
