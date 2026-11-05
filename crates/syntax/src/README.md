# qd-syntax source

Colors the lines of a file diff by language: it finds the grammar from the file's path (or a shebang), highlights each hunk's old and new side with syntect, and returns colored ranges per line. It does not render, does not know gpui, and does not fetch whole files, so a hunk that starts inside a multi-line comment may be colored as code.
