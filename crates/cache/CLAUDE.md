# qc-cache

| Module | Holds |
|---|---|
| `github_cache` | `GitHubCache`: cached pull requests, lists and changed files, with row mapping |
| `ui_state_store` | `UiStateStore`: window placement and review queue state |
| `database_connection` | database path and connection setup (WAL, foreign keys, migrations) |
| `schema_migrations` | append-only `SCHEMA_MIGRATIONS` and their runner (`PRAGMA user_version`) |
| `pull_request_list_key` | `PullRequestListKey` newtype (`REVIEW_REQUESTED`) |
| `window_placement` / `window_mode` | `WindowPlacement`, `WindowMode` (SQL text mapping) |
| `review_queue_view_state` | `ReviewQueueViewState` |

- One database file; each store opens its own connection.
- Schema changes append a new entry to `SCHEMA_MIGRATIONS`; never edit a shipped one.
- Every field is its own typed column; nothing is stored as JSON. Changed files are `pr_file_sets` (version tag) + `pr_files` (one row per file, `position` keeps GitHub's order).
- Adding a field to a `qc-github` type means a migration adding its column and updating the row mapping in `github_cache.rs`.
- UI state tables hold one row (`id = 1`); `review_queue_view.selected_pr_id` has no foreign key (the PR may be gone).
- Changed files are valid only for the `updated_at` in their `pr_file_sets` row.
- Every write is one statement or one transaction (the app relies on this for its poisoned-lock handling).
- Tests use `open_in_memory` (test-only) or a temp dir, never the user's DB.
