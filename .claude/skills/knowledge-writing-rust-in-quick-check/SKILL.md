---
name: knowledge-writing-rust-in-quick-check
description: Use when writing, changing or reviewing Rust in quick-check (.rs files, Cargo.toml, a new crate): the gpui app in crates/app, the GitHub client in crates/github, the SQLite cache in crates/cache.
user-invocable: false
---

# Writing Rust in quick-check

quick-check is a macOS desktop app for reviewing GitHub pull requests, built on gpui. All crates live under `crates/` and share the root Cargo workspace, lockfile and toolchain. These rules apply to new files, modules and crates, and to the lines a change writes. Bringing an existing crate's layout or names up to them is its own refactor commit, not something to fold into a feature. A crate's `CLAUDE.md` adds only what is specific to it; where the two disagree, the crate's file wins for that crate.

Before writing Rust here, read the crate's `CLAUDE.md` and the `README.md` of the folder you work in.

## Module layout

- **A module is named after what it holds**, not after an action or a phase: `review_queue`, `shared_cache`, `schema_migrations`.
- **Put new code where its concept already lives.** A new concept that fits no module gets its own module, named the same way.
- **Every crate's `CLAUDE.md` has a module table:** one row per top-level module and what it holds. A new module adds a row.
- **Crates split by subsystem, not by layer.** `qc-github` talks to GitHub, `qc-cache` persists, `qc-markdown` parses Markdown, `qc-diff` builds file diffs, `qc-syntax` colors them, `quick-check` (the app) owns gpui. Only the app depends on gpui; library crates stay UI-free so they can be tested without a window.

## Files

- **One concept per file.** A file holding several types or unrelated functions is split. When a module has more than one concept, it is a directory.
- **`mod.rs` and the crate root (`lib.rs`, `main.rs`) hold only** the module doc, `mod` declarations and `pub use` re-exports: no logic, constants or types; `main.rs` adds only a short `main` that calls into a module. Re-export what the module offers, so callers write `crate::review_queue::ReviewQueue`, never `crate::review_queue::review_queue::ReviewQueue`.
- **A file holding a type is named after the type** in snake_case: `PullRequestFile` lives in `pull_request_file.rs`, `ReviewQueue` in `review_queue/review_queue.rs`. Repeating the directory name is intended; clippy's `module_inception` lint is allowed. A type's error, result or outcome type used on its own (by another file, or re-exported) gets its own file (`github_error.rs`). A private helper type used only inside one file may stay in it.
- **A file holding one public function is named after the function** (`resolve_token.rs`). A file holding a group of functions, or a group of `impl` methods on one topic, is named after what the group does (`sidebar_rendering.rs`, `detail_rendering.rs`).
- **Generic names are not allowed:** `types.rs`, `utils.rs`/`util/`, `helpers.rs`, `common.rs`, `model.rs`/`models.rs`, `dto.rs`, `shared.rs`, `error.rs`, `client.rs`, `state.rs` (for something that is not a state), `config.rs` or `settings.rs` (say whose: `window_settings.rs`). If a file name would fit in any module, it is too generic.
- **Every folder has a `README.md`:** `src/`, `tests/`, `examples/` and each folder under them (except committed data such as fixtures) has a short plain-English explanation of its responsibility, what it is not responsible for where that is easy to confuse, and how it relates to its neighbours; no file map, no list of types. A new folder gets one, and a change of responsibility updates it.
- **Tests move with their code:** unit tests sit in the file of the code they test. Scenario tests that span a module live in `src/<module>/tests/` (`#[cfg(test)] mod tests;`), one file per subject, named after what they test; the crate-root `tests/` holds only tests of the public API.

## Names

- **Types say what they are, not just their category:** `GitHubClient`, not `Client`; `GitHubError`, not `Error`; `RefreshStatus`, not `Status`. Where the GitHub API has the same concept, use its name (`PullRequestFile`, `review_requested`).
- **Functions say what they do:** `replace_list`, `load_changed_files`; a function named like a query has no side effects.
- **Values with a meaning get a newtype:** a tuple struct over the existing representation (`&'static str`, `u64`), whose doc says what the value is, who produces it and whether it is checked, read through an accessor such as `as_str()` (`PullRequestListKey`).
- **Names are written out in full words.** Variables, parameters, closure arguments, fields and functions say what they hold or do: `pull_request`, not `pr`; `error`, not `e`/`err`; `index`, not `ix`; `row`, not `r`. The only short names allowed: `id` (and `..._id`), `cx` for a gpui context (`App`, `Context<T>`, `AsyncApp`) as gpui's own API names it, names that match an existing type, the standard `len()`/`is_empty()` methods, `i` as the index of a loop that fits on a few lines. Names that leave the crate stay as they are: JSON keys of the GitHub API, SQL column names, environment variables.
- **Names keep their context.** A method says what it acts on, not only the action: a handler is named after the event it handles (`on_refresh`), never just `on_update`.
- **A closure bound to a name is named like a function**, with a verb that says what it does (`find_selected`); predicates start with `is_`, `has_` or `contains_`. Prefer a specific verb over `get_`.

## Inside a file

- **Constants and enums at the top**, before structs, impls and functions.
- **An enum's impl sits with the enum.**
- **Private helpers stay private.** Add accessors instead of widening fields when a sibling file needs a value.
- **No code nothing uses.** Do not add API for a possible future caller; a type, constant, method or trait impl only its own test reads is removed, or gated with `#[cfg(test)]`.
- **Choose with `match` or a named helper, not an if-assignment.** Use `match` when choosing on an enum or a constant; use a named helper when a branch reads or computes something; keep `let x = if cond { a } else { b };` only when both branches are plain values. Do not `match` on a `bool`. Replace number literals with named constants (`SEARCH_PAGE_SIZE`, `SIDEBAR_WIDTH`), and use an enum instead of a `bool` parameter that changes what a function does.
- **Decide in one place, then act.** When code picks one of several actions from a combination of conditions, a small function computes the decision and returns an enum whose variants name each case, and the caller only carries it out. The deciding `match` lists every case, so it never depends on arm order; compare with `cmp` and match on `Ordering` instead of chaining `<` and `==`.
- **Name the parts of what you pass around.** Values that travel together and are read by position (`(u64, String)`, `.0`/`.1`, a returned tuple) get a small struct with named fields.
- **Malformed input is an error, never a panic.** Input from outside the process (an API response, a cache row, a file) is checked; integer conversions use `try_from`, not `as`.
- **`Arc` only for real shared ownership:** tasks or threads that end independently own the same value. Only the owners hold the `Arc` (views, spawned tasks); every function below them borrows. An `Arc` cloned only to get past the borrow checker is restructured first, and when that is not worth it, a comment at the clone says why.
- **A lock guard never lives across a second lock of the same mutex.** A guard taken in a `match` or `if let` scrutinee (`match cache.lock().files(..)?`) lives until the whole expression ends, so locking again in an arm deadlocks: bind the result in its own statement first (`let cached = cache.lock().files(..)?;`).
- **Lock helpers say what a poisoned lock means.** When a poisoned `Mutex` is used as is (`unwrap_or_else(PoisonError::into_inner)`), the helper's doc says why the data stays consistent (`SharedCache::lock`).

## gpui and async code

- **Never block the main thread.** Rendering, event handlers and `cx.spawn` futures run on the main thread. Network calls (`ureq`) and SQLite work run inside `cx.background_executor().spawn(...)`; the foreground task awaits that and writes the result back through the `WeakEntity` it was given. A cache read that is small and local may run on the main thread at startup only.
- **Slow derived data arrives after what it decorates.** Show content as soon as it is usable and add the slow part when ready, in batches spread over the background executor (syntax colors after the diffs), instead of making the first view wait.
- **No second runtime.** The app uses gpui's executors; library crates expose blocking APIs and do not depend on tokio.
- **Hold the `Task` you spawn.** A view stores its in-flight `Task<()>` in a field (`refresh_task`), so starting a new one drops and cancels the old one and closing the view cancels both. `detach()` only for work that must outlive the view, with a comment saying why.
- **A dropped view is not an error.** The result of `this.update(cx, ...)` after an await is ignored with `.ok()`: the view was closed while the task ran.
- **Await events; never poll for them.** No loop that wakes on an interval to recheck a flag; await a channel or a timer set to the next deadline.
- **State changes call `cx.notify()`** in the same method that changes what `render` reads, never in a caller.
- **Content that can grow without bound is a virtualized list of small rows.** A scrolling `div` makes gpui notify its view on every wheel event, so the whole tree is rebuilt and laid out each frame; a `list` (variable heights) or `uniform_list` renders only the rows in view. Model the content as a `Copy` row enum of indexes into the data (`DetailRow`), splice the rows of one part on local changes, and give a pane that scrolls its own view entity so scrolling does not re-render its neighbours (`PullRequestDetail`).
- **`render` only builds elements.** It reads state and calls rendering helpers; it starts no tasks, changes no state and computes nothing expensive. Derived data (a parsed description) is computed when its source changes and kept in the view (`ReviewQueue::set_selected_index`). Long element trees are split into `render_*` methods grouped by area in their own files.
- **Reusable pieces of UI are `RenderOnce` components** (`#[derive(IntoElement)]`) that take what they draw and an `id`, in a module of their own (`MarkdownView`); a view places them, it does not copy their rendering.
- **Content written by other people is untrusted.** Links from GitHub text open only when they are `http(s)://` URLs; nothing from it runs or reads local files.
- **Colors and fonts come from `theme`,** never a hex literal in a view.
- **Preferences live in `AppSettings` and are changed only in the settings window.** The main view stays free of option controls; a view that depends on a preference reads it from the global and observes it (`observe_global::<AppSettings>`), so a change applies everywhere at once (`PullRequestDetail::apply_diff_settings`).
- **A flex item that holds wrapping text along a row's main axis gets `min_w_0()`.** gpui measures text unwrapped when layout asks for its minimum width, and a flex item's default minimum width is that, so without it the item grows past the window instead of wrapping (the detail pane beside the sidebar, a list item's content beside its bullet). Content that must not wrap (code, patches) goes in an `overflow_x_scroll` container with an id, which scrolls instead of widening its parent.

## Persistence

- **Everything stored lives in typed SQLite columns, never in JSON** (or any other self-described format), cached GitHub data included: the schema is our local contract, and SQLite checks it on every write. One table per concept, one column per field (`pull_requests`, `pr_files`); a list is a child table with a `position` column; an enum is a `TEXT` column with a `CHECK` listing its values, mapped by `as_sql_text`/`from_sql_text` beside the enum (`WindowMode`), or beside its store when another crate owns the enum (`diff_option_sql_text.rs` for `qc-diff`'s); one-row state uses `id INTEGER PRIMARY KEY CHECK (id = 1)` (`window_placement`). A value GitHub owns and may extend (a file's `status`) stays unchecked `TEXT`. JSON is only for talking to GitHub: `qc-github` types derive `Deserialize` where they are decoded from a response, and nothing derives `Serialize`.
- **Ask GitHub as little as possible.** Serve from the cache first; when cached data may be stale, check the cheapest marker of change before refetching (a pull request's head and base commits, not its `updated_at`, which moves on every comment), with a conditional request (`If-None-Match` with the stored `ETag`, free when GitHub answers 304); follow every page of a list. Count requests in `ApiUsage` and turn exhausted budgets into `GitHubError::RateLimited` (`fetch_changed_files`).
- **Cached GitHub data expires; user data does not.** Cache rows carry when they were last used and `delete_expired` removes them after the TTL; viewed marks, settings and UI state are never expired.
- **All persistence goes through `qc-cache`,** in the one database file. Each store type opens its own connection (`GitHubCache`, `UiStateStore`); WAL lets them share the file without waiting on each other.
- **A schema change is a new entry in `SCHEMA_MIGRATIONS`,** never an edit of an existing one. Cached GitHub data may be dropped and refetched by a migration; UI state is migrated, not dropped.
- **Restoring never fails the app.** Nothing saved, a row that no longer reads, or a value that no longer fits (a window off every display, a pull request that left the queue) restores nothing for that part and is reported on stderr at most (`window_bounds_from_placement`, `SelectionRestore`).
- **UI state writes are batched** through `UiStateRecorder`: a change starts a short timer, later changes overwrite the pending value, and one task writes in order; the window losing focus writes at once.

## Documentation

- **Describe the effect where the logic happens.** A comment describes what that code does and why; put claims about another module at the code that performs them. Describe the code as it is, without history.
- **Every type and every non-trivial function has a doc comment** in plain words.
- **Every function longer than a few lines gets step comments:** a one-line plain-English `//` comment above each logical step, so the function can be followed by reading only the comments. Say what the step does and, when it is not obvious, why. Explain intent, not syntax; do not restate the code or comment every line.
- **State facts in place.** Never write "see the docs" or "see X" for information the reader needs. Write the fact: the limit, the order, the reason.
- **Name operations in full:** "a list replacement", "the refresh", never a bare verb used as a noun.
- **Module docs** (`//!` in `mod.rs`, `lib.rs` and `main.rs`) list the files of the module and what each holds; update the list when you add, rename or remove a file.

## Tests

- **Tests say what they pin:** every test has a one-line `///` naming the behaviour it checks, and long scenarios have a comment above each phase (set up, act, expect).
- **Test-only code is compiled only for tests:** `#[cfg(test)]`, so release builds cannot call it (`GitHubCache::open_in_memory`).
- **Tests never touch the network or the user's cache.** Parse recorded JSON for GitHub shapes; use an in-memory or temp-dir database.
- **A test that blocks cannot hang:** give every blocking wait a timeout.

## Dependencies

- **Every dependency is declared once in `[workspace.dependencies]`** and inherited with `workspace = true`; a crate adds only features.
- **gpui comes from crates.io** with `default-features = false` and the `font-kit` and `runtime_shaders` features: `runtime_shaders` compiles Metal shaders at launch, so building needs no Metal Toolchain download.

## A new crate

- Add it under `crates/<name>/`, named `qc-<name>` (the binary crate is `quick-check`). The workspace glob picks it up.
- Inherit `version`, `edition`, `license` and `publish` from `[workspace.package]`, and `[lints] workspace = true`.
- Add a crate `CLAUDE.md` with its module table and anything specific to it, and a `README.md` in `src/` and every folder under it. Add its row to the root `CLAUDE.md` crate table.

## Checks before committing

Run in the repository root. All must be clean, with zero warnings.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace -- -D warnings            # without test targets: catches private code only tests use
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace
```

Upstream future-incompatibility notes from gpui's dependencies (`block`, `proc-macro-error2`) are not ours and are ignored. A UI change is also checked by running the app (`cargo run`, or `bacon` to rebuild and relaunch on save).

## Evolving these conventions

This skill is the single source of quick-check's Rust conventions. It grows from review: when a review shows a clear, general signal, turn it into a rule here in the same commit that applies it.

A clear signal is one of:

- a reviewer (person or tool) states a rule outright ("always…", "never…", "we name X like Y");
- the same kind of change is requested in two or more places, or in two reviews;
- the user corrects code you wrote here, and the correction would apply to other code as well.

It is not a signal when a request fixes a single bug, follows from one call site's circumstances, or is a preference stated once without a reason. When in doubt, apply the change and ask whether it should become a rule.

To add or change a rule:

1. Put it in the section it belongs to, as a bold lead-in sentence followed by the reason or the boundary, and one short example from this repository's code.
2. Replace or reword a rule it contradicts; never leave two rules that disagree, and keep the text free of history.
3. Apply it to the code the change touches.
4. Move a rule that holds for one crate only into that crate's `CLAUDE.md` instead.
5. Mention the new or changed rule in the commit message.
