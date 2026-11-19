# qd-markdown

| Module | Holds |
|---|---|
| `markdown_document` | `MarkdownDocument::parse`, the entry point |
| `document_builder` | pulldown-cmark events → blocks (container stack + one open inline text) |
| `markdown_block` | `MarkdownBlock` enum |
| `rich_document` | styled draft projection and source offsets for editing |
| `rich_blocks` | parsed block delimiters and list item spans |
| `container_continuation` | quote markers and indentation for subsequent lines in a nested block |
| `block_format` | editable block prefix kinds |
| `inline_mark` | original inline delimiters and link destinations |
| `fence_delimiters` | matching delimiters around edited literal code |
| `markdown_fence` | source ranges of fenced code and top-level suggestions |
| `inline_text` / `inline_style` / `styled_run` / `link_range` | flattened inline text: one string, sorted non-overlapping styled runs, link ranges |
| `heading_level`, `markdown_list`, `list_item`, `task_state`, `markdown_table`, `column_alignment` | block payload types |
| `html_text` | raw HTML → readable text (`strip_html`, `<br>` detection) |
| `bare_urls` | bare `http(s)://` URLs → link ranges (GitHub autolink literals) |

- UI-free: no gpui. The app's `markdown_view` renders the document.
- Supported: CommonMark + tables, strikethrough, task lists, bare URL autolinks. Raw HTML keeps only its text; images become links labeled with alt text.
- Not supported (yet): `#123` / `owner/repo#123` references and `@mentions` (need repo context), syntax highlighting, footnote definitions (flow in as paragraphs), `> [!NOTE]` alerts.
- Runs are byte ranges on char boundaries, sorted and non-overlapping: gpui's `StyledText` requires that.
- Public-API tests live in `tests/parse_markdown.rs`; helper unit tests sit beside their code.
