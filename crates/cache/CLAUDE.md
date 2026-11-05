# qc-cache

| Module | Holds |
|---|---|
| `github_cache` | `GitHubCache`: cached pull requests, lists and changed files |
| `ui_state_store` | `UiStateStore`: window placement and review queue state |
| `database_connection` | database path and connection setup (WAL, foreign keys, migrations) |
| `schema_migrations` | append-only `SCHEMA_MIGRATIONS` and their runner (`PRAGMA user_version`) |
| `pull_request_list_key` | `PullRequestListKey` newtype (`REVIEW_REQUESTED`) |
| `window_placement` / `window_mode` | `WindowPlacement`, `WindowMode` (SQL text mapping) |
| `review_queue_view_state` | `ReviewQueueViewState` |

- One database file; each store opens its own connection.
- Schema changes append a new entry to `SCHEMA_MIGRATIONS`; never edit a shipped one.
- GitHub rows store the `qc-github` type as JSON in `data` next to indexed columns; all other state is typed columns.
- UI state tables hold one row (`id = 1`); `review_queue_view.selected_pr_id` has no foreign key (the PR may be gone).
- Derived data (`pr_files`) is valid only for the `updated_at` it was fetched at.
- Every write is one statement or one transaction (the app relies on this for its poisoned-lock handling).
- Tests use `open_in_memory` (test-only) or a temp dir, never the user's DB.
