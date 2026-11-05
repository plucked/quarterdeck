# qd-syntax

| Module | Holds |
|---|---|
| `diff_syntax` | `DiffSyntax::highlight(diff, path)`, the entry point; spans per (hunk, line) |
| `syntax_highlighter` | shared `SyntaxSet` (bat's grammars via `two-face`, newline mode) + the token theme in a `OnceLock`; language detection; per-line highlighting |
| `token_theme` | `token_theme()`: a syntect theme whose "colors" are markers, one per `SyntaxKind` (scope selectors: comments, keywords/storage minus operators, strings, constants, functions, types, invalid); `kind_of_marker` reads them back. Plain text gets no span |
| `syntax_span` / `syntax_kind` | byte range of a line + its `SyntaxKind`; the kinds |

- syntect uses the pure-Rust `regex-fancy` backend (no oniguruma C build).
- Each hunk's old side (context + removed) and new side (context + added) is highlighted in order with fresh state; context and added lines take new-side colors.
- Limits: lines over 2000 bytes stay plain; files over 5000 diff lines stay plain. Plain text (variables, operators, punctuation) has no span.
- Spans carry kinds, not colors: the app colors them from the active color theme at draw time, so a theme change needs no re-highlighting.
- First use loads the grammars (~30 ms): call from the background executor.
