# quick-check (app)

| Module | Holds |
|---|---|
| `launch_app` | gpui start-up and the main window |
| `app_commands` | actions (`Quit`, `Refresh`), key bindings, menu bar |
| `app_services` | `AppServices`: GitHub client (or reason it is missing) + cache |
| `shared_cache` | `SharedCache`: `Arc<Mutex<GitHubCache>>` with poison-tolerant `lock` |
| `theme` | colors and fonts |
| `review_queue` | main view: review-requested list + selected PR detail and diffs |

- Key bindings: `cmd-r` refresh, `cmd-q` quit.
- Flow: show cached queue at launch → refresh from GitHub in background → cache. Selecting a PR loads its files from cache if current, else fetches and caches.
- UI changes are verified by running the app; view logic that can be tested without a window lives in plain types (`SelectionAfterRefresh`).
