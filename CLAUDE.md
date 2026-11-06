# Quarterdeck

macOS desktop app (Rust + gpui) for reviewing GitHub pull requests efficiently: the viewer's own and review-requested ones, including GitHub stacks (layer by layer or combined), with line comments that either form a GitHub review (approve / request changes / comment) or stay local to copy for an AI agent. Personal tool shared with colleagues: macOS on Apple silicon only, distributed as signed, notarized GitHub releases that update themselves.

## Workflow
- Remote `origin`: `git@github.com:plucked/quarterdeck.git` (public). Work directly on `main` and push it. Small atomic commits, prefixed `feat:`/`fix:`/`refactor:`/`docs:`/`chore:`/`style:`/`test:`.
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
| `qd-syntax` | `crates/syntax` | syntax colors for diff hunks (`syntect` + bat's grammars via `two-face`, Graphite theme) |
| `qd-diff` | `crates/diff` | per-file patch → hunks, side-by-side rows, inline word/char changes, whitespace folding (`similar`) |
| `qd-file-tree` | `crates/file_tree` | changed files as a folder tree (shared file order), test/generated detection, file filters |
| `qd-video` | `crates/video` | movie frames decoded by macOS AVFoundation into BGRA (the startup splash) |
| `qd-update` | `crates/update` | self-update: release manifest (`latest.json`), download with SHA-256 check, signature check, bundle swap, relaunch |

Only the app depends on gpui. Library crates are blocking and UI-free; the app runs them on `cx.background_executor()`. No tokio.

## Run & check
```bash
bacon                                       # rebuild + relaunch on save (job `app`, from the dev bundle with icon); UI state is restored
cargo run                                   # needs GITHUB_TOKEN or `gh auth login`
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace
```

App bundle: `packaging/macos/bundle.sh` → `target/bundle/Quarterdeck.app` (`aarch64-apple-darwin` only, ad hoc signed); `--dev` keeps `target/debug/Quarterdeck.app` (debug binary hard-linked, icon remade only when `AppIcon.png` changes, startup movie copied into `Resources`), which bacon's `app` job runs so the app has its icon and name (`de.plucked.quarterdeck`, `Info.plist` template, version from `[workspace.package]`, `AppIcon.icns` from the 1024 px `AppIcon.png`). Started from Finder the app gets launchd's environment: no `GITHUB_TOKEN`, no Homebrew `PATH`.

Release: the user-invoked skill `/release [patch|minor|major|X.Y.Z]` (`.claude/skills/release/`) picks the version (minor with a `feat:` since the last tag, else patch), writes user-facing notes, bumps and pushes, then runs `packaging/macos/release.sh [--notes FILE] [--dry-run]`, run locally (no CI): checks (clean tree, HEAD pushed, gh's own login can push (gh runs without `GITHUB_TOKEN`/`GH_TOKEN`, which may hold a token that cannot create releases), tag `v<version>` unused, notary profile `quarterdeck-notary`, movie not an LFS pointer), `bundle.sh`, re-signs with `Developer ID Application: Rainer Heynke (97ZZEC8R43)` (hardened runtime, timestamp; no entitlements needed), notarizes + staples the app (zip) and the DMG, writes `latest.json` (version, notes, `zip_url`, `zip_sha256`, `zip_size`, via `plutil`), and publishes GitHub release `v<version>` as a draft first, then public, so `releases/latest/download/latest.json` always has its zip. Output in `target/release-artifacts/<version>/`. Notes default to the `feat:`/`fix:` subjects since the last `v*` tag.

Devbox: `devbox.json` pins rustup, bacon, gh, sqlite; `devbox run app | dev | check | test`. `devbox.lock` pins the resolved versions (Devbox 0.19); `devbox update` moves them.

## Facts
- gpui 0.2.2 from crates.io, `default-features = false`, features `font-kit` + `runtime_shaders` (no Metal Toolchain needed to build).
- Token: `GITHUB_TOKEN`, else the token saved in the login Keychain from Settings → GitHub (generic password, service `de.plucked.quarterdeck`, account `github-token`; checked with `GET /user` first; the app restarts to use it), else `gh auth token` (`gh` on `PATH`, `/opt/homebrew/bin`, `/usr/local/bin`). Without one the app shows cached data only.
- Updates: `AppUpdater` checks `github.com/plucked/quarterdeck/releases/latest/download/latest.json` (no API budget) 15 s after launch and every 4 h; a newer version is downloaded to `~/Library/Caches/de.plucked.quarterdeck/updates`, SHA-256 and size checked, unpacked with `ditto` into `.Quarterdeck.app-update` beside the bundle, verified (`codesign` against the Developer ID requirement of team `97ZZEC8R43`, `spctl` for notarization, `CFBundleShortVersionString`), and installed by two renames on quit or on Restart to update (relaunch by a detached `sh` that waits for the process to exit, then `open`s the bundle). Off when not running from a release-signed bundle (dev builds, `cargo run`), when translocated, or when the bundle's folder is not writable.
- A database migrated past the app's `SCHEMA_MIGRATIONS` (a newer app ran) is refused; startup failures show in an `osascript` alert, since a Finder launch has no terminal.
- Database: `~/Library/Application Support/Quarterdeck/cache.sqlite` for release builds; debug builds (bacon, `cargo run`) use `…/Quarterdeck Dev/cache.sqlite`, started once from a `VACUUM INTO` copy of the release one, so a development build ahead by a migration never upgrades the installed release's database (which would then refuse it). Delete a folder to reset that build's cache and UI state. The app was called quick-check before; a release build moves `…/quick-check` to `…/Quarterdeck` once.
- Everything stored is typed SQLite columns, cached GitHub data included; never JSON (see the skill's Persistence section).
- Queue refresh: at launch, `cmd-r`, every 5 min after the last refresh, on window activation when older than 1 min, and when the watched repositories change. Two search requests (mine, review requested), one more per 10 watched repositories and page of 100 (others' PRs: `is:pr is:open archived:false -author:@me repo:…`, at most 5 pages each), and one GraphQL request (stack memberships of all three lists; the search API lacks them) each. Labels come with the search results (`pr_labels`).
- Others tab (`cmd-3`): open PRs by others in the watched repositories (Settings → Repositories: `owner/name` or a GitHub link, checked with `GET /repos/{o}/{r}`; quick picks from the queue's repositories; a preference in `AppSettings`). Every watched repository shows a header, even with nothing shown; its filter button opens an inline filter under it: Hide drafts, Hide review requests, Updated within (any / 7 / 30 days), Labels and Authors chips (click: only those; ⌥-click: hide; counts given the rest). Filters are per repository UI state (`others_filters` + `others_filter_chips`, saved only while they narrow something), applied locally, so they cost no requests. A failed others' fetch keeps the last list and shows `Others: …` in the tab.
- GitHub requests are minimized: a file's whole text (for the lines around hunks) only on the first expand click, by blob SHA (one REST request; by path in the head commit, one GraphQL request, when the SHA is abbreviated or unknown), cached by blob SHA for good (content addressed); cached files are reused while the PR's commits are unchanged (checked with a conditional request, free on 304); stacks are checked with a conditional request; a combined stack diff is one `.diff` download per commit pair, cached for good (commits never change); unused cached data expires after the TTL (Settings → GitHub, default 1 day).
- Stacks: GitHub stacked PRs (GA Oct 2026). Membership from GraphQL `PullRequest.stack`/`stackEntry`; layers from `GET /repos/{o}/{r}/stacks/{number}` (bottom first, each targets the layer below). Search/issues ids differ from pulls ids (`StackLayer::pull_request_id`): match layers by number.
- Lints: clippy `pedantic` (warn, `-D warnings` in checks), `module_inception`/`missing_errors_doc`/`must_use_candidate` allowed, `unsafe_code = "deny"` (one exception: `qd-video`'s `video_reader.rs`, the AVFoundation calls).
- Startup: the main window opens hidden and see-through while `StartupSplash` plays `crates/app/assets/startup-intro.mp4` (Git LFS, `*.mp4`; copied into the bundle's `Contents/Resources`), so the queue loads and refreshes underneath; frames are paced by the display's refreshes, not timers; at the end (or a click) the main window draws its first frame, then the splash fades out over 1 s as the main window fades in. Without the movie (a clone without LFS has a pointer file) the main window shows at once.
