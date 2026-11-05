# qc-syntax

| Module | Holds |
|---|---|
| `diff_syntax` | `DiffSyntax::highlight(diff, path)`, the entry point; spans per (hunk, line) |
| `syntax_highlighter` | shared `SyntaxSet` (bat's grammars via `two-face`, newline mode) + `TwoDark` theme in a `OnceLock`; language detection; per-line highlighting |
| `syntax_span` / `syntax_color` | colored byte range of a line; RGB |

- syntect uses the pure-Rust `regex-fancy` backend (no oniguruma C build).
- Each hunk's old side (context + removed) and new side (context + added) is highlighted in order with fresh state; context and added lines take new-side colors.
- Limits: lines over 2000 bytes stay plain; files over 5000 diff lines stay plain. Spans in the theme's default foreground are omitted.
- First use loads the grammars (~30 ms): call from the background executor.
