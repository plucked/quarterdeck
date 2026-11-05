# quick-check

macOS desktop app (Rust + gpui) for reviewing GitHub pull requests efficiently: the viewer's own and review-requested ones, including GitHub stacks (layer by layer or combined). Personal tool; macOS only for now.

## Workflow
- Local only: no remote, work directly on `main`. Small atomic commits, prefixed `feat:`/`fix:`/`refactor:`/`docs:`/`chore:`/`style:`/`test:`.
- Rust conventions: skill `knowledge-writing-rust-in-quick-check` (`.claude/skills/`). Load it before touching `.rs` or `Cargo.toml`.
- Keep this file and each crate's `CLAUDE.md` current in the same commit as the change. Compact: facts only.

## Crates
| Crate | Path | Holds |
|---|---|---|
| `quick-check` | `crates/app` | gpui binary: window, views, background tasks, UI state restore |
| `qc-github` | `crates/github` | blocking GitHub REST client (`ureq`, plus one GraphQL query for stacks) and its data shapes |
| `qc-cache` | `crates/cache` | the local SQLite database: cached GitHub data + UI state, with migrations |
| `qc-markdown` | `crates/markdown` | GitHub-flavored Markdown → typed document (`pulldown-cmark`) |
| `qc-syntax` | `crates/syntax` | syntax colors for diff hunks (`syntect` + bat's grammars via `two-face`, `TwoDark` theme) |
| `qc-diff` | `crates/diff` | per-file patch → hunks, side-by-side rows, inline word/char changes, whitespace folding (`similar`) |
| `qc-file-tree` | `crates/file_tree` | changed files as a folder tree (shared file order), test/generated detection, file filters |

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

## Facts
- gpui 0.2.2 from crates.io, `default-features = false`, features `font-kit` + `runtime_shaders` (no Metal Toolchain needed to build).
- Token: `GITHUB_TOKEN`, else `gh auth token`. Without one the app shows cached data only.
- Database: `~/Library/Application Support/quick-check/cache.sqlite` (delete to reset cache and UI state).
- Everything stored is typed SQLite columns, cached GitHub data included; never JSON (see the skill's Persistence section).
- Queue refresh: at launch, `cmd-r`, every 5 min after the last refresh, and on window activation when older than 1 min. Two search requests (mine, review requested) and one GraphQL request (stack memberships; the search API lacks them) each.
- GitHub requests are minimized: cached files are reused while the PR's commits are unchanged (checked with a conditional request, free on 304); stacks are checked with a conditional request; a combined stack diff is one `.diff` download per commit pair, cached for good (commits never change); unused cached data expires after the TTL (Settings → GitHub, default 1 day).
- Stacks: GitHub stacked PRs (GA Oct 2026). Membership from GraphQL `PullRequest.stack`/`stackEntry`; layers from `GET /repos/{o}/{r}/stacks/{number}` (bottom first, each targets the layer below). Search/issues ids differ from pulls ids (`StackLayer::pull_request_id`): match layers by number.
- Lints: clippy `pedantic` (warn, `-D warnings` in checks), `module_inception`/`missing_errors_doc`/`must_use_candidate` allowed, `unsafe_code = "deny"`.
