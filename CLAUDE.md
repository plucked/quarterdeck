# Quarterdeck

macOS desktop app (Rust + gpui) for reviewing GitHub pull requests efficiently: the viewer's own and review-requested ones, including GitHub stacks (layer by layer or combined), with line comments that either form a GitHub review (approve / request changes / comment) or stay local to copy for an AI agent. Personal tool; macOS only for now.

## Workflow
- Remote `origin`: `git@github.com:plucked/quarterdeck.git` (private). Work directly on `main` and push it. Small atomic commits, prefixed `feat:`/`fix:`/`refactor:`/`docs:`/`chore:`/`style:`/`test:`.
- Rust conventions: skill `knowledge-writing-rust-in-quarterdeck` (`.claude/skills/`). Load it before touching `.rs` or `Cargo.toml`.
- Keep this file and each crate's `CLAUDE.md` current in the same commit as the change. Compact: facts only.
- `README.md` describes the features and how to run the app for people; update it when a user-facing feature or the way to run it changes.

## Crates
| Crate | Path | Holds |
|---|---|---|
| `quarterdeck` | `crates/app` | gpui binary: window, views, background tasks, UI state restore |
| `qd-github` | `crates/github` | blocking GitHub client (`ureq`): REST, plus GraphQL for stacks and reviews (threads, submit, edit, resolve); its data shapes |
| `qd-cache` | `crates/cache` | the local SQLite database: cached GitHub data + UI state, with migrations |
| `qd-markdown` | `crates/markdown` | GitHub-flavored Markdown → typed document (`pulldown-cmark`) |
| `qd-syntax` | `crates/syntax` | syntax colors for diff hunks (`syntect` + bat's grammars via `two-face`, `TwoDark` theme) |
| `qd-diff` | `crates/diff` | per-file patch → hunks, side-by-side rows, inline word/char changes, whitespace folding (`similar`) |
| `qd-file-tree` | `crates/file_tree` | changed files as a folder tree (shared file order), test/generated detection, file filters |

Only the app depends on gpui. Library crates are blocking and UI-free; the app runs them on `cx.background_executor()`. No tokio.

## Run & check
```bash
bacon                                       # rebuild + relaunch on save (job `app`); UI state is restored
cargo run                                   # needs GITHUB_TOKEN or `gh auth login`
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace
```

Devbox: `devbox.json` pins rustup, bacon, gh, sqlite; `devbox run app | dev | check | test`. Commit `devbox.lock` once it exists (it is created on the first Devbox run, which needs Nix).

## Facts
- gpui 0.2.2 from crates.io, `default-features = false`, features `font-kit` + `runtime_shaders` (no Metal Toolchain needed to build).
- Token: `GITHUB_TOKEN`, else `gh auth token`. Without one the app shows cached data only.
- Database: `~/Library/Application Support/Quarterdeck/cache.sqlite` (delete to reset cache and UI state). The app was called quick-check before; opening the default database moves `…/quick-check` there once.
- Everything stored is typed SQLite columns, cached GitHub data included; never JSON (see the skill's Persistence section).
- Queue refresh: at launch, `cmd-r`, every 5 min after the last refresh, and on window activation when older than 1 min. Two search requests (mine, review requested) and one GraphQL request (stack memberships; the search API lacks them) each.
- GitHub requests are minimized: cached files are reused while the PR's commits are unchanged (checked with a conditional request, free on 304); stacks are checked with a conditional request; a combined stack diff is one `.diff` download per commit pair, cached for good (commits never change); unused cached data expires after the TTL (Settings → GitHub, default 1 day).
- Stacks: GitHub stacked PRs (GA Oct 2026). Membership from GraphQL `PullRequest.stack`/`stackEntry`; layers from `GET /repos/{o}/{r}/stacks/{number}` (bottom first, each targets the layer below). Search/issues ids differ from pulls ids (`StackLayer::pull_request_id`): match layers by number.
- Lints: clippy `pedantic` (warn, `-D warnings` in checks), `module_inception`/`missing_errors_doc`/`must_use_candidate` allowed, `unsafe_code = "deny"`.
