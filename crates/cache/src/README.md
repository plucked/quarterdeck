# qc-cache source

Keeps a local SQLite copy of what the app fetched from GitHub, so a launch shows the last known review queue before the network answers and the app stays usable offline. It owns the schema and its migrations. It does not fetch anything and does not decide when data is stale beyond comparing a pull request's `updated_at`; the app refreshes and writes back.
