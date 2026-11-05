# qd-syntax source

Sorts the tokens of a file diff by kind: it finds the grammar from the file's path (or a shebang), highlights each hunk's old and new side with syntect, and returns ranges per line marked comment, keyword, string, constant, definition, type or invalid. Choosing colors is the app's job (from its active color theme), so the same result serves every theme. It does not render, does not know gpui, and does not fetch whole files, so a hunk that starts inside a multi-line comment may be read as code.
