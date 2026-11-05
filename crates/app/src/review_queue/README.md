# review_queue

The main window's view: the list of pull requests that request the viewer's review on the left, and the selected pull request with its changed files on the right. It decides when to read the cache, when to refresh from GitHub and what stays selected; it does not know how either is done, which `qc-cache` and `qc-github` handle.
