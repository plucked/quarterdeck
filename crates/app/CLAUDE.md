# quick-check (app)

| Module | Holds |
|---|---|
| `launch_app` | gpui start-up; opens the main window at its saved placement or centered |
| `main_window` | `MainWindow` root view: owns `ReviewQueue`, records window placement, writes UI state on focus loss |
| `app_commands` | actions (`Quit`, `OpenSettings`, `CloseWindow`, `Refresh`, `Copy`), key bindings, menu bar |
| `app_settings` | `AppSettings` global: the preferences (diff view settings), loaded at launch, saved on change, observed by views |
| `settings_window` | settings window (cmd-,, single instance): category list + page per `SettingsCategory`, `SettingRow` component |
| `app_services` | `AppServices`: GitHub client (or reason it is missing) + cache |
| `scrollbar` | `VerticalScrollbar`: reusable draggable scrollbar for a `ScrollbarTarget` (`ScrollHandle` or `ListState`; gpui draws none) |
| `shared_cache` | `SharedCache`: `Arc<Mutex<GitHubCache>>` with poison-tolerant `lock` |
| `ui_state` | `UiStateRecorder` global (reads saved UI state, batches writes) and window placement conversion |
| `diff_view` | per-row diff rendering (file header, hunk header, unified line, split row, note, card framing) and `DiffSummaryBar` |
| `segmented_control` | `SegmentedControl`: reusable row of exclusive options |
| `markdown_view` | `MarkdownView`: reusable component rendering a `qc_markdown::MarkdownDocument`, with mouse text selection and copy |
| `theme` | colors and fonts |
| `review_queue` | main view: review-requested list, selection, refresh schedule, restore; owns the detail view |
| `pull_request_detail` | `PullRequestDetail` view: header, description and files as one virtualized `list` of `DetailRow`s; loads files, builds/colors diffs, settings, viewed/collapsed |

- Key bindings: `cmd-,` settings, `cmd-w` close window (settings window handles it), `cmd-r` refresh, `cmd-c` copy (handled by the focused view, e.g. a `MarkdownView` with a selection), `cmd-q` quit.
- Settings: a new preference goes in `AppSettings` (field + setter that saves), gets a `SettingRow` on its category's page (new categories: a `SettingsCategory` variant + page), and views read it with `AppSettings::…(cx)` and `observe_global::<AppSettings>`. The main view shows no option controls.
- Markdown: parse once when the source changes (`ReviewQueue::set_selected_index` parses the description), keep `Arc<MarkdownDocument>`, render with `MarkdownView::new(id, document.clone())`. Links open only if `http(s)://`; the text comes from other people.
- Markdown selection: state via `window.use_keyed_state` keyed by (view id, `Arc` pointer of the document), so a new document starts unselected; keep the `Arc` stable while the source is unchanged (`ParsedDescription`). Each frame records every text's `TextLayout` for hit testing; drags are tracked by window-wide mouse listeners registered in a `canvas` paint. Copy joins texts with `\n`. A press released without a drag is a link click.
- Diffs: built on the background executor when files load and when whitespace/highlight settings change (layout/wrapping only redraw); `LoadedFile` holds `Arc`s of file + diff. `FileReviewState` owns viewed/collapsed per file; viewed marks persist with a `PatchFingerprint` and reset when the patch changes. Files over 400 changed lines and viewed files start collapsed.
- Syntax colors: after load and after a rebuild, `highlight_syntax` runs one background batch per core and applies each batch as it finishes (matched by the diff's `Arc`, so a rebuilt diff ignores stale colors). Files draw plain until then. A real 99-file / 4.5k-line PR takes ~0.7 s single-threaded; `code_highlights` merges syntax colors with changed-range backgrounds into non-overlapping runs.
- Flow: show cached queue at launch → refresh from GitHub in background → cache. Selecting a PR loads its files from cache if current, else fetches and caches.
- Restore: window placement, selected PR (matched against the cached queue, then the first refreshed one, else dropped), detail scroll as `DetailScrollPosition` (row + offset; applied when the files load, so no heights are needed).
- Performance: the detail pane renders only visible rows (`list` with `measure_all`, overdraw 600 px). Collapse/viewed toggles `splice` one file's rows; settings/load `reset` and re-anchor on the file at the top. Benchmark (100-file PR, split + wrap, 45 px/frame): 273 ms → 8.3 ms per frame. Scroll mode scrolls each file sideways via a per-file offset on horizontal gestures (rows are separate, so no per-file scroll container).
- Scrollbars: wrap the scroll container in a `relative()` div with the container `size_full()` and a `VerticalScrollbar` beside it: `ScrollbarTarget::Handle` for a div or `uniform_list` (`handle.0.borrow().base_handle.clone()`), `ScrollbarTarget::List` for a `list`. The thumb is painted at paint time.
- Wrapping text in a flex row needs `min_w_0()` on its flex item (see the skill's gpui rules); the detail pane and Markdown list items have it.
- Window display: gpui reports and takes window bounds relative to the window's display, so the placement saves the display's UUID (`display.uuid()`, stable across reconnects) and `launch_app` reopens with `display_id`; a missing display falls back to the main one, and placements that do not fit their display open centered.
- gpui on macOS opens windows by content size but reports `window_bounds` as the frame (title bar included): save `viewport_size` for windowed mode or the window grows each relaunch.
- UI changes are verified by running the app; view logic that can be tested without a window lives in plain types (`SelectionAfterRefresh`, `SelectionRestore`, `RefreshSchedule`).
