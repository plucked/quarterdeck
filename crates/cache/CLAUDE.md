# qc-cache

| Module | Holds |
|---|---|
| `github_cache` | `GitHubCache`: connection, every read and write |
| `schema_migrations` | append-only `SCHEMA_MIGRATIONS` and their runner (`PRAGMA user_version`) |
| `pull_request_list_key` | `PullRequestListKey` newtype (`REVIEW_REQUESTED`) |

- Schema changes append a new entry to `SCHEMA_MIGRATIONS`; never edit a shipped one.
- Rows store the `qc-github` type as JSON in `data` next to indexed columns (`id`, `repo`, `number`, `updated_at`).
- Derived data (`pr_files`) is valid only for the `updated_at` it was fetched at.
- Every write is one statement or one transaction (the app relies on this for its poisoned-lock handling).
- Tests use `open_in_memory` (test-only) or a temp dir, never the user's DB.
