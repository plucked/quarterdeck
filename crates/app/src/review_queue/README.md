# review_queue

The main window's view: the list of pull requests that request the viewer's review, which one is selected, and when the list refreshes from GitHub. It hands the selected pull request to `pull_request_detail`, which shows it; it does not load files or draw diffs. It also decides what stays selected across refreshes and relaunches.
