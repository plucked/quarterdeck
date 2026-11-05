# diff_view

Draws what `qc-diff` built for each changed file: a header (status, path, line counts, a viewed checkbox) and a body in unified or side-by-side layout with line numbers and marked changes, wrapping or scrolling long lines. Also draws the toolbar that switches those options. It does not parse patches, fetch files or keep any state: the view that places it owns which files are collapsed or viewed and the settings, and passes handlers for changing them.
