# quick-check

macOS desktop app (Rust + gpui) for reviewing GitHub pull requests efficiently. Personal tool; macOS only for now.

## Workflow
- Local only: no remote, work directly on `main`. Small atomic commits, prefixed `feat:`/`fix:`/`refactor:`/`docs:`/`chore:`/`style:`/`test:`.
- Rust conventions: skill `knowledge-writing-rust-in-quick-check` (`.claude/skills/`). Load it before touching `.rs` or `Cargo.toml`.
- Keep this file and each crate's `CLAUDE.md` current in the same commit as the change. Compact: facts only.

## Crates
| Crate | Path | Holds |
|---|---|---|
| `quick-check` | `crates/app` | gpui binary: window, views, background tasks, UI state restore |
| `qc-github` | `crates/github` | blocking GitHub REST client (`ureq`) and its data shapes |
| `qc-cache` | `crates/cache` | the local SQLite database: cached GitHub data + UI state, with migrations |

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
- Our own state is typed SQLite columns, never JSON (see the skill's Persistence section).
- Queue refresh: at launch, `cmd-r`, every 5 min after the last refresh, and on window activation when older than 1 min.
- Lints: clippy `pedantic` (warn, `-D warnings` in checks), `module_inception`/`missing_errors_doc`/`must_use_candidate` allowed, `unsafe_code = "deny"`.
