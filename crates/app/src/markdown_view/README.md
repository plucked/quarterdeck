# markdown_view

Draws a Markdown document that `qd-markdown` already parsed: blocks become gpui elements, inline styles become text highlights, links open in the browser, and the text can be selected and copied. It does not parse Markdown and does not decide where documents come from; a view parses once, keeps the document, and places a `MarkdownView` wherever it should appear. Only web links (`http`, `https`) open; others are shown but inert. Cutting a text at the edges of styled ranges for highlights lives here too, and the diff's code highlights use it.
