# markdown_view

Draws a Markdown document that `qc-markdown` already parsed: blocks become gpui elements, inline styles become text highlights, and links open in the browser. It does not parse Markdown and does not decide where documents come from; a view parses once, keeps the document, and places a `MarkdownView` wherever it should appear. Only web links (`http`, `https`) open; others are shown but inert.
