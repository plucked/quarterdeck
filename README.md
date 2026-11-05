# Quarterdeck

A fast native macOS app for reviewing GitHub pull requests: the ones you opened and the ones waiting for your review, including GitHub's stacked pull requests. Written in Rust on [gpui](https://www.gpui.rs).

## Features

**Your review queue**
- Two lists: **Mine** (pull requests you opened) and **Review** (those asking for your review), with counts.
- Grouped by repository, and stacked pull requests grouped as stacks with every layer in order. Groups fold, and stay folded across launches.
- Each row shows whether it is a draft, its number, author and how long ago it changed. Layers asking for your review carry a Review badge.
- Refreshes on its own (at launch, every 5 minutes, and when you come back to the window after a while) or with ⌘R.

**Stacked pull requests**
- Review a stack layer by layer, or combined: every change from the bottom of the stack up to the layer you are on, as one diff.
- Step through layers with `{` and `}`, switch between layer and combined with `c`, and ⇧-click a layer in the sidebar to start the combined range there.

**Reading diffs**
- Unified or side-by-side diffs, syntax highlighting, changed words highlighted inside lines, wrapping or sideways scrolling, and whitespace changes folded away (all in Settings).
- A file tree beside the diff with added and removed lines per file and folder, and a badge for added, removed and moved files.
- Filters by kind (code, tests, generated, moved) and by file type, with live counts, and a switch to hide viewed files.
- Mark files viewed (`v` marks the current one and jumps to the next); marks are kept, and drop off when a file changes again.
- Large and generated files start folded. Pull requests with thousands of files stay responsive, including those over GitHub's 3,000-file list limit (loaded as one diff).

**Commenting and reviewing**
- Comment on a line or a range of lines (double-click a line, or the `+` beside it), and reply to, edit, delete and resolve GitHub conversations.
- Each comment goes either to GitHub, as a draft in your next review, or stays local.
- Submit a GitHub review: comment, approve or request changes, with a summary.
- Local notes copy as Markdown, with the file, the lines and the code around them, ready to paste to an AI agent.

**Acting on a pull request**
- Merge with any merge method the repository allows, as admin when you may bypass its rules, or through its merge queue. Merging a stack layer merges the layers below it too.
- Close a pull request, and delete its branch.

**Light on GitHub**
- Everything is cached locally, so the app opens instantly and works offline with what it last saw.
- Unchanged pull requests cost no requests: GitHub's "not modified" answers are free, and stacks and combined diffs are fetched once per commit.

## Run it

You need macOS, [Rust](https://rustup.rs) (stable; `rust-toolchain.toml` picks it), and access to GitHub through either the [GitHub CLI](https://cli.github.com) or a token:

```bash
gh auth login              # or: export GITHUB_TOKEN=<a token with repo access>
cargo run --release
```

Quarterdeck uses `GITHUB_TOKEN` when it is set, otherwise the GitHub CLI's login. Without either it shows what it has cached.

### As a Mac app

```bash
packaging/macos/bundle.sh               # builds target/bundle/Quarterdeck.app
packaging/macos/bundle.sh --universal   # one app for Apple Silicon and Intel Macs
```

Drag `Quarterdeck.app` into Applications. It is signed for this Mac only (not notarized yet). Started from Finder or the Dock it does not see your terminal's `GITHUB_TOKEN` or `PATH`, so for now start it with `open target/bundle/Quarterdeck.app` from a terminal where `gh` or the token works.

### With Devbox

With [Devbox](https://www.jetify.com/devbox), the tools come pinned in a shell of their own (Rust through `rustup`, bacon, the GitHub CLI, SQLite); Xcode's command line tools still have to be installed on the Mac:

```bash
devbox run app             # build and start Quarterdeck
devbox run dev             # rebuild and relaunch on every save (bacon)
devbox run check           # format, lint, test and docs checks
devbox shell               # a shell with the tools, for anything else
```

The first run installs Nix if it is missing (Devbox asks for your password) and fetches the tools, at the versions `devbox.lock` pins.

### Working on it

For working on Quarterdeck itself, [bacon](https://dystroy.org/bacon) rebuilds and relaunches on every save (as a small app bundle, so it has its icon and name), and the app reopens where it was:

```bash
bacon
```

## Keyboard

| Keys | Action |
|---|---|
| ⌘1 / ⌘2 | Mine / Review |
| ⌘R | Refresh |
| ] / [ | Next / previous file |
| v | Mark the current file viewed and go to the next |
| } / { | Next / previous stack layer |
| c | Layer ↔ combined stack review |
| ⌘B | Show or hide the side panel |
| ⌃⌘S | Show or hide the sidebar |
| ⌘, | Settings |

## Data

The cache and what the app remembers (viewed files, notes, layout) live in `~/Library/Application Support/Quarterdeck/`. Settings → GitHub shows how much space it takes and clears the cache. Deleting the folder resets everything.
