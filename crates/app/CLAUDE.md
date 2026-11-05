# quick-check (app)

| Module | Holds |
|---|---|
| `launch_app` | gpui start-up; opens the main window at its saved placement or centered |
| `main_window` | `MainWindow` root view: owns `ReviewQueue`, records window placement, writes UI state on focus loss |
| `app_commands` | actions (`Quit`, `Refresh`, `Copy`), key bindings, menu bar |
| `app_services` | `AppServices`: GitHub client (or reason it is missing) + cache |
| `shared_cache` | `SharedCache`: `Arc<Mutex<GitHubCache>>` with poison-tolerant `lock` |
| `ui_state` | `UiStateRecorder` global (reads saved UI state, batches writes) and window placement conversion |
| `markdown_view` | `MarkdownView`: reusable component rendering a `qc_markdown::MarkdownDocument`, with mouse text selection and copy |
| `theme` | colors and fonts |
| `review_queue` | main view: review-requested list + selected PR detail and diffs; refresh schedule; selection restore |

- Key bindings: `cmd-r` refresh, `cmd-c` copy (handled by the focused view, e.g. a `MarkdownView` with a selection), `cmd-q` quit.
- Markdown: parse once when the source changes (`ReviewQueue::set_selected_index` parses the description), keep `Arc<MarkdownDocument>`, render with `MarkdownView::new(id, document.clone())`. Links open only if `http(s)://`; the text comes from other people.
- Markdown selection: state via `window.use_keyed_state` keyed by (view id, `Arc` pointer of the document), so a new document starts unselected; keep the `Arc` stable while the source is unchanged (`ParsedDescription`). Each frame records every text's `TextLayout` for hit testing; drags are tracked by window-wide mouse listeners registered in a `canvas` paint. Copy joins texts with `\n`. A press released without a drag is a link click.
- Flow: show cached queue at launch → refresh from GitHub in background → cache. Selecting a PR loads its files from cache if current, else fetches and caches.
- Restore: window placement, selected PR (matched against the cached queue, then the first refreshed one, else dropped), detail scroll (applied once files load; gpui clamps it).
- gpui on macOS opens windows by content size but reports `window_bounds` as the frame (title bar included): save `viewport_size` for windowed mode or the window grows each relaunch.
- UI changes are verified by running the app; view logic that can be tested without a window lives in plain types (`SelectionAfterRefresh`, `SelectionRestore`, `RefreshSchedule`).
