# quick-check (app)

| Module | Holds |
|---|---|
| `launch_app` | gpui start-up; opens the main window at its saved placement or centered |
| `main_window` | `MainWindow` root view: owns `ReviewQueue`, records window placement, writes UI state on focus loss |
| `app_commands` | actions (`Quit`, `Refresh`, `Copy`), key bindings, menu bar |
| `app_services` | `AppServices`: GitHub client (or reason it is missing) + cache |
| `scrollbar` | `VerticalScrollbar`: reusable draggable scrollbar for any `ScrollHandle` (gpui draws none) |
| `shared_cache` | `SharedCache`: `Arc<Mutex<GitHubCache>>` with poison-tolerant `lock` |
| `ui_state` | `UiStateRecorder` global (reads saved UI state, batches writes) and window placement conversion |
| `diff_view` | `FileDiffView` (header with status/rename/counts/viewed, unified or split body, wrap or scroll) and `DiffToolbar` |
| `segmented_control` | `SegmentedControl`: reusable row of exclusive options |
| `markdown_view` | `MarkdownView`: reusable component rendering a `qc_markdown::MarkdownDocument`, with mouse text selection and copy |
| `theme` | colors and fonts |
| `review_queue` | main view: review-requested list + selected PR detail and diffs; refresh schedule; selection restore |

- Key bindings: `cmd-r` refresh, `cmd-c` copy (handled by the focused view, e.g. a `MarkdownView` with a selection), `cmd-q` quit.
- Markdown: parse once when the source changes (`ReviewQueue::set_selected_index` parses the description), keep `Arc<MarkdownDocument>`, render with `MarkdownView::new(id, document.clone())`. Links open only if `http(s)://`; the text comes from other people.
- Markdown selection: state via `window.use_keyed_state` keyed by (view id, `Arc` pointer of the document), so a new document starts unselected; keep the `Arc` stable while the source is unchanged (`ParsedDescription`). Each frame records every text's `TextLayout` for hit testing; drags are tracked by window-wide mouse listeners registered in a `canvas` paint. Copy joins texts with `\n`. A press released without a drag is a link click.
- Diffs: built on the background executor when files load and when whitespace/highlight settings change (layout/wrapping only redraw); `LoadedFile` holds `Arc`s of file + diff. `FileReviewState` owns viewed/collapsed per file; viewed marks persist with a `PatchFingerprint` and reset when the patch changes. Files over 400 changed lines and viewed files start collapsed.
- Flow: show cached queue at launch → refresh from GitHub in background → cache. Selecting a PR loads its files from cache if current, else fetches and caches.
- Restore: window placement, selected PR (matched against the cached queue, then the first refreshed one, else dropped), detail scroll (applied once files load; gpui clamps it).
- Scrollbars: wrap the scroll container in a `relative()` div with the container `size_full()` and a `VerticalScrollbar` beside it; for a `uniform_list` pass `handle.0.borrow().base_handle.clone()`. The thumb is painted from the handle at paint time; dragging it does not fire the container's scroll-wheel listener.
- Wrapping text in a flex row needs `min_w_0()` on its flex item (see the skill's gpui rules); the detail pane and Markdown list items have it.
- gpui on macOS opens windows by content size but reports `window_bounds` as the frame (title bar included): save `viewport_size` for windowed mode or the window grows each relaunch.
- UI changes are verified by running the app; view logic that can be tested without a window lives in plain types (`SelectionAfterRefresh`, `SelectionRestore`, `RefreshSchedule`).
