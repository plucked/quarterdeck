# brief_view

How a brief written by an AI agent looks: the status card while there is none, the lead, every studio widget (sections, callouts, quoted code, tables and traces, the scope, the reading order, graphs and sequences), the page tabs and navigation, the list of widgets left out, and the footer.

It is drawn by free functions, not a view, so two views share it: the pull request detail shows the brief of the selected pull request in place of its diff, and the brief gallery shows every widget on sample data for working on the layout. The view passes in the diffs code is quoted from and what clicks do.

It does not run agents or check their answers; `qd-studio` does that, and the pull request detail starts the runs.
