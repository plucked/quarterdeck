# quick-check (app)

| Module | Holds |
|---|---|
| `launch_app` | gpui start-up; opens the main window at its saved placement or centered |
| `main_window` | `MainWindow` root view: owns `ReviewQueue`, records window placement, writes UI state on focus loss |
| `app_commands` | actions (`Quit`, `Refresh`), key bindings, menu bar |
| `app_services` | `AppServices`: GitHub client (or reason it is missing) + cache |
| `shared_cache` | `SharedCache`: `Arc<Mutex<GitHubCache>>` with poison-tolerant `lock` |
| `ui_state` | `UiStateRecorder` global (reads saved UI state, batches writes) and window placement conversion |
| `theme` | colors and fonts |
| `review_queue` | main view: review-requested list + selected PR detail and diffs; refresh schedule; selection restore |

- Key bindings: `cmd-r` refresh, `cmd-q` quit.
- Flow: show cached queue at launch → refresh from GitHub in background → cache. Selecting a PR loads its files from cache if current, else fetches and caches.
- Restore: window placement, selected PR (matched against the cached queue, then the first refreshed one, else dropped), detail scroll (applied once files load; gpui clamps it).
- gpui on macOS opens windows by content size but reports `window_bounds` as the frame (title bar included): save `viewport_size` for windowed mode or the window grows each relaunch.
- UI changes are verified by running the app; view logic that can be tested without a window lives in plain types (`SelectionAfterRefresh`, `SelectionRestore`, `RefreshSchedule`).
