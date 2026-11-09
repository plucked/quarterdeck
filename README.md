# Quarterdeck

A fast native macOS app for reviewing GitHub pull requests: the ones you opened and the ones waiting for your review, including GitHub's stacked pull requests. Written in Rust on [gpui](https://www.gpui.rs).

## Features

**Starting up**
- A short intro movie plays while Quarterdeck loads your queue and syncs with GitHub, then fades into the app. Click it to skip. Its bottom right corner shows the version.

**Your review queue**
- Three lists: **Mine** (pull requests you opened), **Review** (those asking for your review, and those you already reviewed that are still open) and **Others** (everyone else's open pull requests in repositories you choose), with counts.
- Grouped by repository, and stacked pull requests grouped as stacks with every layer in order. Groups fold, and stay folded across launches.
- Each row shows whether it is a draft, its number, author, how long ago it changed, the branch it goes into and the one it comes from (`main ← feat/login`), and how you last reviewed it: a green check (approved), a red cross (changes requested), a ring with … (commented) or a dash (your review was dismissed); hover it for the words. Layers asking for your review carry a Review badge.
- Refreshes on its own (at launch, every 5 minutes, and when you come back to the window after a while) or with ⌘R.

**Following other people's work**
- Choose repositories in Settings → Repositories (type `owner/name` or paste a GitHub link, or pick one your queue already has), and the Others tab lists their open pull requests that you did not open.
- Each repository has its own filter, opened from the button on its header: hide drafts, hide the ones already asking for your review, only those updated in the last 7 or 30 days, and labels (with their GitHub colors) and authors (with their avatars) as chips (click to show only those, ⌥-click to hide), with counts that say what each chip would show; chips that would show nothing are left out. Filters are kept, and the header shows `4 / 12` while one narrows the list.
- Hide pull requests by branch in all three lists at once (Settings → Filters): add a pattern such as `feat/legacy-*` (`*` matches anything, `?` one character) and pick whether it looks at the branch a pull request comes from, the one it goes into, or either.

**Stacked pull requests**
- Review a stack layer by layer, or combined: every change from the bottom of the stack up to the layer you are on, as one diff.
- Step through layers with `{` and `}`, switch between layer and combined with `c`, and ⇧-click a layer in the sidebar to start the combined range there.

**Reading diffs**
- Eight color themes made for reading code, four light and four dark (Settings → Appearance, with a live preview; Graphite by default): every color readable against its background, and diffs in blue (added) and orange (removed), which stay distinct with red-green colour blindness. Interface in Schibsted Grotesk, code in JetBrains Mono (both built in).
- Show the unchanged code above, below and between the changes: 20 lines at a time or all of it, from the bar over each gap. The file is fetched once, on the first click, and kept.
- Unified or side-by-side diffs, syntax highlighting, changed words highlighted inside lines, wrapping or sideways scrolling, and whitespace changes folded away (all in Settings).
- A file tree beside the diff with added and removed lines per file and folder, and a badge for added, removed and moved files. Fold folders one by one, or all at once with Collapse all / Expand all. The tree icon in a file's diff header jumps to that file in the tree.
- Filters by kind (code, tests, generated, moved) and by file type, with live counts, and a switch to hide viewed files.
- Mark files viewed (`v` marks the current one and jumps to the next); marks are kept, and drop off when a file changes again.
- Large and generated files start folded. Pull requests with thousands of files stay responsive, including those over GitHub's 3,000-file list limit (loaded as one diff).

**Commenting and reviewing**
- Comment on a line or a range of lines (double-click a line, or the `+` beside it), and reply to, edit, delete and resolve GitHub conversations.
- Each comment goes either to GitHub, as a draft in your next review, or stays local.
- Submit a GitHub review: comment, approve or request changes, with a summary.
- Local notes copy as Markdown, with the file, the lines and the code around them, ready to paste to an AI agent.

**Briefs written by Claude (experimental)**
- Switch a pull request from **Diff** to **Brief** and click **Write brief**: Claude Code reads the change's diffs and walks you through it the way a colleague would: why it exists, the context you need, what changes and what stays the same, the order to read it in, how it works, and its limits. It explains; it does not review or judge the change.
- Before writing, say how well you know the area: **I know this code well** gives just the change, **I know the area roughly** explains the mechanisms it touches, and **I'm new to this area** starts with background on that part of the system, read from the surrounding code. Quarterdeck remembers the answer per repository until you quit.
- Briefs are kept: a ✦ in the sidebar marks pull requests that have one, and a brief says above it which commit it was written for, and warns with **Write again** when the pull request has moved on since.
- Something wrong or weird in a brief? **Report…** at the right of the line above it saves your words with everything about how the brief was made (prompt, instructions, Claude's raw answer, what it read) to `~/Library/Application Support/Quarterdeck/brief-reports/`. Hand the folder's `report.md` to a Claude Code session in this repository to improve the brief tool. Code in the brief is quoted from the diff itself, and every quote opens the diff at that line. A larger change reads like a short presentation: pages from the overview through each mechanism to what to check, with tabs to jump and buttons to the next page. Where a picture explains more, the brief draws diagrams: who owns what and where data goes (before and after, when the change reshapes it), state machines, and message sequences.
- Claude gets a shallow checkout of the pull request's head commit (Git LFS files left out), plus its diff, to follow callers, types and tests. It can only read files in that checkout, which is deleted when the brief is done; it cannot run or change anything. The code it reads is sent to Anthropic.
- Needs [Claude Code](https://claude.ai/code) installed and logged in, and `git` (Xcode's command line tools). Briefs are kept until you quit.
- **Window → Brief Gallery** shows every part a brief can have on sample text, without running Claude.

**Acting on a pull request**
- From the side panel's Actions tab: merge with any merge method the repository allows, as admin when you may bypass its rules, or through its merge queue. Merging a stack layer merges the layers below it too.
- Close a pull request, and delete its branch.

**Light on GitHub**
- Everything is cached locally, so the app opens instantly and works offline with what it last saw.
- Unchanged pull requests cost no requests: GitHub's "not modified" answers are free, and stacks and combined diffs are fetched once per commit. A file's full text is fetched only when you expand around its changes, and once per version of the file.

## Install

Download `Quarterdeck-<version>.dmg` from the [latest release](https://github.com/plucked/quarterdeck/releases/latest), open it and drag Quarterdeck into Applications. It needs a Mac with Apple silicon and macOS 13 or later; it is signed and notarized, so it opens like any other app.

**Signing in.** Quarterdeck uses the [GitHub CLI](https://cli.github.com)'s login when `gh` is installed and logged in (`gh auth login`). Otherwise open Settings → GitHub, paste a [personal access token](https://github.com/settings/tokens/new?scopes=repo,read:org&description=Quarterdeck) with the `repo` and `read:org` scopes and click Sign in: it is checked with GitHub, saved in your Keychain, and the app restarts with it.

**Updates.** Quarterdeck looks for a new version shortly after launch and every few hours, downloads and checks it in the background (it must be signed by the same developer and notarized), and installs it when you quit. To install it at once, click **Restart to update** in the top bar, or in Settings → Updates, which also shows what's new. Quarterdeck → Check for Updates… checks right away. Updates need the app to be in a folder you can write to, such as Applications.

## Run it from source

You need macOS, [Rust](https://rustup.rs) (stable; `rust-toolchain.toml` picks it), [Git LFS](https://git-lfs.com) for the intro movie (without it the app starts without the intro), and access to GitHub through either the [GitHub CLI](https://cli.github.com) or a token:

```bash
gh auth login              # or: export GITHUB_TOKEN=<a token with repo access>
cargo run --release
```

Quarterdeck uses `GITHUB_TOKEN` when it is set, then a token saved in Settings → GitHub, then the GitHub CLI's login (found on `PATH` or where Homebrew installs it). Without any it shows what it has cached. Builds from source don't update themselves.

### As a Mac app

```bash
packaging/macos/bundle.sh               # builds target/bundle/Quarterdeck.app (Apple silicon)
```

It is signed for this Mac only; releases are signed and notarized by the release script.

### Releasing

Releases are built, signed, notarized and published from a Mac with the Developer ID certificate, no CI:

```bash
packaging/macos/release.sh --dry-run    # build and sign only, to try it
packaging/macos/release.sh              # release the version in Cargo.toml
```

Bump `version` under `[workspace.package]` in `Cargo.toml`, commit and push, then run the script. It builds the app, signs it (Developer ID, hardened runtime), has Apple notarize the app and the disk image, writes the release notes from the `feat:` and `fix:` commits since the last release (or takes `--notes FILE`), and publishes the GitHub release `v<version>` with the zip, the disk image and `latest.json`, which running apps read to update. The script's header lists the one-time setup (the notary credentials).

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

`bacon gallery` does the same and opens the brief gallery each time, for working on how briefs look.

## Keyboard

| Keys | Action |
|---|---|
| ⌘1 / ⌘2 / ⌘3 | Mine / Review / Others |
| ⌘R | Refresh |
| ] / [ | Next / previous file |
| v | Mark the current file viewed and go to the next |
| } / { | Next / previous stack layer |
| c | Layer ↔ combined stack review |
| ⌘B | Show or hide the side panel |
| ⌃⌘S | Show or hide the sidebar |
| ⌘, | Settings |

## Data

The cache and what the app remembers (viewed files, notes, layout) live in `~/Library/Application Support/Quarterdeck/`. Development builds (`bacon`, `cargo run`) keep their own copy in `Quarterdeck Dev/`, started from the installed app's data the first time, so trying a newer build never changes what the installed app uses. Settings → GitHub shows how much space it takes and clears the cache. Deleting the folder resets everything.
