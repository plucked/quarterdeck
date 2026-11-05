# qd-markdown source

Parses the Markdown GitHub uses (pull request descriptions, comments) into a small typed document: blocks such as paragraphs, lists and tables, each holding its text already flattened into one string with styled ranges and links. It does not render anything and does not know gpui; the app's `markdown_view` turns the document into elements. Raw HTML is not rendered: only its readable text is kept.
