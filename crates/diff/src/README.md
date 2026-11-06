# qd-diff source

Turns one file's unified patch into what a diff view shows: hunks of lines numbered on both sides, rows for a side-by-side view, and the changed words or characters inside each changed line. It can treat lines that differ only in whitespace as unchanged. Given the file's text after the change, it also fills the gaps between hunks with the unchanged lines asked for. It does not fetch patches or file texts, compute diffs of whole files, or render anything; GitHub supplies the patch and the app's `diff_view` draws the result.
