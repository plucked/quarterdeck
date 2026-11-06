# qd-cache

| Module | Holds |
|---|---|
| `github_cache` | `GitHubCache`: cached pull requests, lists and changed files, with row mapping |
| `ui_state_store` | `UiStateStore`: window placement and review queue state |
| `database_connection` | database path (`Quarterdeck/`, or `Quarterdeck Dev/` in debug builds, seeded once by `VACUUM INTO` from the release database, into a temporary file moved into place) and connection setup (WAL, foreign keys, migrations); `prepare_data_directory` runs before the default database opens |
| `watched_repositories_storage` | the repositories the Others tab lists (`watched_repositories`, ordered by `position`; a preference, never expired) |
| `others_filter` / `updated_within` / `others_filter_storage` | `OthersFilter` (hide drafts, hide review requests, `UpdatedWithin`, label and author `FacetFilter<String>`s) per repository (`RepositoryOthersFilter`); stored in `others_filters` + `others_filter_chips` (facet `label`/`author`, choice `only`/`hidden`), a row only while the filter narrows something |
| `schema_migrations` | append-only `SCHEMA_MIGRATIONS` and their runner (`PRAGMA user_version`) |
| `pull_request_list_key` | `PullRequestListKey` newtype (`REVIEW_REQUESTED`) |
| `window_placement` / `window_mode` / `display_uuid` | `WindowPlacement` (bounds relative to its display), `WindowMode` (SQL text mapping), `DisplayUuid` |
| `file_set_storage` / `cached_file_set` / `file_set_revision` | changed files with the commits + PR ETag they were fetched for; `file_set` (marks used), `put_files`, `confirm_files` |
| `file_text_storage` | `file_texts`: whole file contents by full blob SHA; `file_text(sha)` takes a full or abbreviated (≥ 7 hex) SHA as a key range `sha..sha+'g'` (an index seek; `LIKE` would scan), and an abbreviation two blobs share finds none; `put_file_text` |
| `cache_cleanup` / `cleanup_counts` | `delete_expired(ttl)` (unused file sets; PRs in no list), `clear()` (then `VACUUM` and a WAL checkpoint, so the file shrinks) |
| `cache_usage` / `cache_usage_storage` | `CacheUsage` (+ `CachedData`: entries, files, bytes) per kind (pull requests, PR files, comparisons), size on disk (pages + WAL) and free pages; `GitHubCache::usage` sums text bytes with `LENGTH(CAST(… AS BLOB))` |
| `cache_ttl` / `cache_settings_storage` | `CacheTtl` (6 h, 1 d, 3 d, 1 w; default 1 d) and its row |
| `color_theme` / `appearance_settings_storage` | `ColorTheme` (Ledger, Vellum, Overcast, Signal, Graphite (default), Tide, Lantern, Beacon), stored by name in `appearance_settings` (one row) |
| `sql_integers` / `unix_now` | checked integer conversions; current time |
| `review_queue_view_state` / `detail_scroll_position` | `ReviewQueueViewState`: selection + detail scroll as row and offset, tab, sidebar visibility |
| `diff_view_settings` / `diff_layout` / `line_wrapping` / `diff_option_sql_text` / `diff_view_settings_storage` | diff view settings (one row) and their SQL text, incl. the `qd-diff` enums |
| `viewed_file` / `patch_fingerprint` / `viewed_files_storage` / `review_scope` | viewed marks per `ReviewScope` (a PR, or a combined range of stack layers) and file, with an FNV-1a patch fingerprint (stable across Rust versions) |
| `review_note` / `note_id` / `note_destination` / `note_anchor` / `review_summary` / `review_notes_storage` | the user's review comments: `ReviewNote` (local note or GitHub draft) per `ReviewScope`, on lines (`NoteAnchor::Line`) or replying to a GitHub thread (`NoteAnchor::Reply`); `NoteId` made by the app (time-based, unique per process) so writes can be batched; `ReviewSummary` (verdict + body) per PR |
| `panel_widths` / `panel_widths_storage` | `PanelWidths` (sidebar, side panel; defaults 380 / 300): one row, each width saved on its own |
| `folded_groups` / `folded_groups_storage` | `FoldedGroups` (repositories, `FoldedStack`s): the folded sidebar groups in `folded_repositories` + `folded_stacks`, replaced whole in one transaction |
| `queue_tab` / `stack_review_mode` | `QueueTab` (authored, review requested) in the queue state; `StackReviewMode` (layer, combined) in the review layout |
| `review_layout_state` / `panel_visibility` / `review_layout_storage` | `ReviewLayoutState` (one row): file tree `PanelVisibility`, `hide_viewed`, hidden `qd_file_tree::FileKind`s (`hide_code`, `hide_tests`, `hide_generated`, `hide_unchanged_moves`), stack review mode, comment destination |
| `stack_storage` / `cached_stack` | `pr_stacks` + `pr_stack_layers`: a stack's layers with the response ETag |
| `comparison_storage` | `compare_file_sets` + `compare_files`: files between two commits (combined stack diffs) |

- One database file; each store opens its own connection.
- Pull requests keep their labels in `pr_labels` (position order, cascade-deleted with the pull request). Migration 20 added it, the Others tables and `queue_tab = 'others'` (the column re-made with the wider CHECK, its value kept).
- Schema changes append a new entry to `SCHEMA_MIGRATIONS`; never edit a shipped one. A database whose `user_version` is past the known migrations (a newer app ran) is refused, not opened.
- Every field is its own typed column; nothing is stored as JSON. Changed files are `pr_file_sets` (version tag) + `pr_files` (one row per file, `position` keeps GitHub's order).
- Adding a field to a `qd-github` type means a migration adding its column and updating the row mapping in `github_cache.rs`.
- UI state tables hold one row (`id = 1`); `review_queue_view.selected_pr_id`, `viewed_files.pr_id` and `combined_viewed_files` have no foreign key (the PR may be gone or refetched; the user's state stays). Combined marks are keyed by repo + first and last layer number.
- `pull_requests` carries the stack membership (`stack_*` columns, all NULL for a standalone PR). `put_pull_request`/`pull_request(id)` hold a stack layer opened outside any list.
- `review_notes`: one table for both destinations; scope is `pr_id` or (repo, first, last layer numbers), enforced by CHECKs; anchor is (side, line, start_line) or `reply_to_thread`. GitHub drafts are deleted by the app as GitHub takes them.
- Comparisons never go stale (commits are immutable); comparisons and stacks expire by `last_used_at` like file sets. So do file texts (content addressed by blob SHA, shared by every pull request and version with that content). `pr_files.sha` / `compare_files.sha`: migration 19 dropped the file sets and comparisons cached without them, so every cached file has its SHA (NULL only for deleted files and renames without an `index` line). `file_set_commits` reads a set's commits without its files.
- A file set is current for the `updated_at` in its `pr_file_sets` row; an older one is still valid when the PR's head/base commits match (`confirm_files`). `last_used_at` moves on every read and write and drives expiry.
- Every write is one statement or one transaction (the app relies on this for its poisoned-lock handling).
- Tests use `open_in_memory` (test-only) or a temp dir, never the user's DB.
