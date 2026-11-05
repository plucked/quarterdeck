# qc-cache source

Keeps a local SQLite copy of what the app fetched from GitHub (pull requests, lists, changed files, stacks and combined diffs), so a launch shows the last known review queue before the network answers and the app stays usable offline, and the UI state it restores after a relaunch. It owns the schema and its migrations. It does not fetch anything and does not decide when data is stale beyond comparing a pull request's `updated_at` and the commits a file set was fetched for; the app refreshes and writes back.
