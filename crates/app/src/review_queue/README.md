# review_queue

The main window's view: the viewer's own pull requests and those that request their review (one tab each, with a stack's layers grouped under its header), which one is selected, and when the lists refresh from GitHub. It hands the selected pull request to `pull_request_detail`, which shows it, and opens the stack layers the detail asks for, fetching one that is in neither list; it does not load files or draw diffs. It also decides what stays selected across refreshes and relaunches.
