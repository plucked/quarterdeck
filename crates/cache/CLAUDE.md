# qc-cache

| Module | Holds |
|---|---|
| `github_cache` | `GitHubCache`: cached pull requests, lists and changed files, with row mapping |
| `ui_state_store` | `UiStateStore`: window placement and review queue state |
| `database_connection` | database path and connection setup (WAL, foreign keys, migrations) |
| `schema_migrations` | append-only `SCHEMA_MIGRATIONS` and their runner (`PRAGMA user_version`) |
| `pull_request_list_key` | `PullRequestListKey` newtype (`REVIEW_REQUESTED`) |
| `window_placement` / `window_mode` / `display_uuid` | `WindowPlacement` (bounds relative to its display), `WindowMode` (SQL text mapping), `DisplayUuid` |
| `file_set_storage` / `cached_file_set` / `file_set_revision` | changed files with the commits + PR ETag they were fetched for; `file_set` (marks used), `put_files`, `confirm_files` |
| `cache_cleanup` / `cleanup_counts` | `delete_expired(ttl)` (unused file sets; PRs in no list), `clear()` |
| `cache_ttl` / `cache_settings_storage` | `CacheTtl` (6 h, 1 d, 3 d, 1 w; default 1 d) and its row |
| `sql_integers` / `unix_now` | checked integer conversions; current time |
| `review_queue_view_state` / `detail_scroll_position` | `ReviewQueueViewState`: selection + detail scroll as row and offset |
| `diff_view_settings` / `diff_layout` / `line_wrapping` / `diff_option_sql_text` / `diff_view_settings_storage` | diff view settings (one row) and their SQL text, incl. the `qc-diff` enums |
| `viewed_file` / `patch_fingerprint` / `viewed_files_storage` / `viewed_scope` | viewed marks per `ViewedScope` (a PR, or a combined range of stack layers) and file, with an FNV-1a patch fingerprint (stable across Rust versions) |
| `panel_widths` / `panel_widths_storage` | `PanelWidths` (sidebar, side panel; defaults 380 / 300): one row, each width saved on its own |
| `queue_tab` / `stack_review_mode` | `QueueTab` (authored, review requested) in the queue state; `StackReviewMode` (layer, combined) in the review layout |
| `review_layout_state` / `panel_visibility` / `review_layout_storage` | `ReviewLayoutState` (one row): file tree `PanelVisibility`, `hide_viewed`, hidden `qc_file_tree::FileKind`s (`hide_code`, `hide_tests`, `hide_generated`, `hide_unchanged_moves`), stack review mode |
| `stack_storage` / `cached_stack` | `pr_stacks` + `pr_stack_layers`: a stack's layers with the response ETag |
| `comparison_storage` | `compare_file_sets` + `compare_files`: files between two commits (combined stack diffs) |

- One database file; each store opens its own connection.
- Schema changes append a new entry to `SCHEMA_MIGRATIONS`; never edit a shipped one.
- Every field is its own typed column; nothing is stored as JSON. Changed files are `pr_file_sets` (version tag) + `pr_files` (one row per file, `position` keeps GitHub's order).
- Adding a field to a `qc-github` type means a migration adding its column and updating the row mapping in `github_cache.rs`.
- UI state tables hold one row (`id = 1`); `review_queue_view.selected_pr_id`, `viewed_files.pr_id` and `combined_viewed_files` have no foreign key (the PR may be gone or refetched; the user's state stays). Combined marks are keyed by repo + first and last layer number.
- `pull_requests` carries the stack membership (`stack_*` columns, all NULL for a standalone PR). `put_pull_request`/`pull_request(id)` hold a stack layer opened outside any list.
- Comparisons never go stale (commits are immutable); comparisons and stacks expire by `last_used_at` like file sets.
- A file set is current for the `updated_at` in its `pr_file_sets` row; an older one is still valid when the PR's head/base commits match (`confirm_files`). `last_used_at` moves on every read and write and drives expiry.
- Every write is one statement or one transaction (the app relies on this for its poisoned-lock handling).
- Tests use `open_in_memory` (test-only) or a temp dir, never the user's DB.
