# qc-diff source

Turns one file's unified patch into what a diff view shows: hunks of lines numbered on both sides, rows for a side-by-side view, and the changed words or characters inside each changed line. It can treat lines that differ only in whitespace as unchanged. It does not fetch patches, compute diffs of whole files, or render anything; GitHub supplies the patch and the app's `diff_view` draws the result.
