# brief_view

How a brief written by an AI agent looks: the status card while there is none, the lead, every studio widget (sections, callouts, quoted code, tables and traces, the scope, the reading order, graphs and sequences), the page tabs and navigation, the list of widgets left out, and the footer.

It is drawn by free functions, not a view, so two views share it: the pull request detail shows the brief of the selected pull request in place of its diff, and the brief gallery shows every widget on sample data for working on the layout. The view passes in the diffs code is quoted from and what clicks do.

It does not run agents or check their answers; `qd-studio` does that, and the pull request detail starts the runs.

A rewrite keeps the saved brief visible. The status card and compact replacement row distinguish agent generation from saving an accepted brief. A failed save offers Retry save for the same completed brief and Discard, while Write again starts a new generation. Active saves show Saving brief and Cancel; duplicate rewrite controls are disabled. MCP briefs compare both captured head and base revisions. A successful replacement resets measurements and the report panel even when its row positions match the earlier brief.
