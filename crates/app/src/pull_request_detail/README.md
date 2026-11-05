# pull_request_detail

The right pane for one pull request: header, description, diff toolbar and every changed file, laid out as a single virtualized list of small rows (a file header, a hunk header, one diff line), so only what is on screen is built. It loads the files (cache first, then GitHub), builds and colors their diffs off the main thread, and keeps the diff settings and each file's viewed and collapsed state. It does not choose which pull request to show or keep the queue; `review_queue` tells it, and it reports when it scrolls so the position can be saved.
