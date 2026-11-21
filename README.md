# Quarterdeck

A fast native macOS app for reviewing GitHub pull requests: the ones you opened and the ones waiting for your review, including GitHub's stacked pull requests. Written in Rust on [gpui](https://www.gpui.rs).

## Features

**Starting up**
- A short intro movie plays while Quarterdeck loads your queue and syncs with GitHub, then fades into the app. Click it to skip, or launch with `--no-splash` to open immediately without the movie or fade. Its bottom right corner shows the version.
- A small dot beside the app name shows when Quarterdeck is loading a pull request or coloring syntax. Hover over it to see the current step.
- Your previous selection is restored when available. Otherwise, startup opens **Mine** and selects its first stack, or its first standalone pull request when there are no stacks.

**Your review queue**
- Three lists: **Mine** (pull requests you opened), **To review** (open PRs with a pending request for you personally, including repeat requests) and **Others** (everyone else's open pull requests in repositories you choose), with counts.
- PRs are grouped by repository and stack. GitHub stacks and PRs linked by matching base and head branches share the same queue rows. A single chain reads straight down from base to tip. When a parent has multiple children, the rail forks and siblings align at the same depth. **To review** shows the whole stack when any layer needs your review. Other layers are dimmed and can still be opened. Only pending requests count toward the tab total. Groups fold, and stay folded across launches.
- Each row shows whether it is a draft, its number, author, how long ago it changed, the branch it goes into and the one it comes from (`main ← feat/login`), and how you last reviewed it: a green check (approved), a red cross (changes requested), a ring with … (commented) or a dash (your review was dismissed); hover it for the words. A pending request on a PR you previously reviewed is labeled **Review again**. Requested stack layers carry a **Review** or **Review again** badge.
- Refreshes on its own (at launch, every 5 minutes, and when you come back to the window after a while) or with ⌘R. An unchanged review keeps your scroll position, file search and folded files.

**Following other people's work**
- Choose repositories in Settings → Repositories (type `owner/name` or paste a GitHub link, or pick one your queue already has), and the Others tab lists their open pull requests that you did not open.
- Each repository has its own filter, opened from the button on its header: hide drafts, hide the ones already asking for your review, only those updated in the last 7 or 30 days, and labels (with their GitHub colors) and authors (with their avatars) as chips (click to show only those, ⌥-click to hide), with counts that say what each chip would show; chips that would show nothing are left out. Filters are kept, and the header shows `4 / 12` while one narrows the list.
- Hide pull requests by branch in all three lists at once (Settings → Filters): add a pattern such as `feat/legacy-*` (`*` matches anything, `?` one character) and pick whether it looks at the branch a pull request comes from, the one it goes into, or either.

**Stacked pull requests**
- Review one layer or combine the path from the lowest unmerged ancestor through the selected PR. Sibling branches stay outside that diff. At a fork, choose the next child in the sidebar.
- In **Diff** and **Guided review**, the selected tab extends to show **Layer / Combined**. Summary and Brief tuck those options away. Choose individual layers in the sidebar.
- Step through layers with `{` and `}`, switch between layer and combined with `c`, and ⇧-click a layer in the sidebar to start the combined range there.

**Reading diffs**
- The middle pane has **Summary**, **Diff**, **Guided review** and **Brief** tabs in the pull request header. Click the PR title to open it on GitHub. Hover over the title to see it in full with the author's avatar and username. The title and head branch use the space beside the tabs and end in an ellipsis when they run out of room. Hovering either one fades the tabs out while the text expands. The branch preview also reveals the base branch. Summary shows the pull request's description, while Diff remains the default.
- **Focus Mode** (⇧⌘F or the framing icon on the right of the top bar) gives the current view the full width of the window. The PR header and side panels return when you leave Focus Mode.
- The side pane uses the same rounded tabs for **Files**, **Review**, **Actions** and **Checks**. Its bubble expands to show **GitHub / Local** in Review or **Merge / Close** in Actions, with motion that respects Reduce Motion.
- Eight color themes made for reading code, four light and four dark (Settings → Appearance, with a live preview; Graphite by default): every color readable against its background, and diffs in blue (added) and orange (removed), which stay distinct with red-green colour blindness. Interface in Schibsted Grotesk, code in JetBrains Mono (both built in).
- Show the unchanged code above, below and between the changes: 20 lines at a time or all of it, from the bar over each gap. The file is fetched once, on the first click, and kept.
- Unified or side-by-side diffs, syntax highlighting, changed words highlighted inside lines, wrapping or sideways scrolling, and whitespace changes folded away (all in Settings).
- A file tree beside the diff with an icon for each file's type (languages, configs, Docker and Git files, Unity assets and more), added and removed lines per file and folder, each folder's changed-file count including its subfolders, and a badge for added, removed and moved files. Fold folders one by one, or all at once with Collapse all / Expand all. The tree icon in a file's diff header jumps to that file in the tree.
- Optional folder heatmap (Settings → Appearance, on by default): folder names and counts go from cold blue to warm orange as more files change, using colors matched to each theme. Counts include moves and binary changes that match the current filters. Hide viewed removes reviewed files from folder counts, line totals and heat; collapsing folders keeps their matching descendants counted.
- Filters by kind (code, tests, generated, moved) and by file type, with live counts, and a switch to hide viewed files.
- **Focus Drawer**: the **Diff** tab uses the file tree; the separate **Guided review** tab uses an ordered Guide navigator. Search paths with fuzzy matching or choose **Contents** to search complete text at the loaded revision, including unchanged lines; click a matching line to read its surrounding source. Coverage counts searched text files, unavailable files (including binaries and oversized texts), and failures. Kind and Type filters live behind the filter button; guide navigation lives in Guided review mode. A horizontal viewed control combines the count and progress bar, with Hide viewed, Reset viewed and Undo. A folder’s Viewed checkbox marks or unmarks only files matching the active filters, including matches in collapsed subfolders.
- **Guided reviews**: choose the main **Guided review** tab, then **Create guided review** to have local Claude Code (Opus, high effort) organize the PR or combined stack range into ordered buckets and a suggested reading order for steps and the files inside each group. Single-file and grouped steps share one guided diff workspace with the review note above their files. Files start expanded; marking one viewed collapses its diff. Related assets or mechanical files can form one review item with a normal checkbox, so hundreds of binaries need only one stop. The Diff tab returns to ordinary browsing, and returning to Guided review restores the selected step, folds and scroll. Each bucket can be marked viewed at once. **Mark group viewed** immediately opens the next unfinished item in the guided order, continuing across buckets. Guided reviews survive relaunch and cache cleanup; **Refresh guide** replaces the guide while preserving viewed marks. Outdated guides keep exact saved membership, and new files appear as ungrouped until refreshed. Existing guides retain their saved items and file order; refresh them to apply the new grouping and ordering instructions. Generation shows progress and can be cancelled. Needs Claude Code installed and logged in; like briefs, its read-only run sends the code it reads to Anthropic. Claude now builds the guide through a private per-run MCP connection: overlap or missing-file errors return directly to the agent for repair before anything is saved. The prior guide survives cancellation, validation failures and save errors. If saving fails after generation, **Retry save** saves the completed guide without another Claude run; **Discard** removes that unsaved draft. Unsaved drafts last for this session and are discarded if the reviewed commits change.
- Mark files viewed (`v` marks the current one and jumps to the next); marks are kept, and drop off when a file changes again. **Reset viewed** beside the Files progress label clears every viewed mark in the current PR or combined range, including files hidden by filters.
- The top file's header stays visible while scrolling its diff, keeping its name, tree button and Viewed checkbox within reach.
- Press ⌘F to find text in the current file’s diff, including added lines, removed lines and any context you’ve expanded. Return advances to the next match, Shift-Return goes back, and Escape closes Find.
- Large and generated files start folded. Pull requests with more than 500 files try one diff download first, falling back to the paginated files API if GitHub has not prepared the diff yet. Pull requests beyond the API's 3,000-file limit still need a complete diff; an incomplete fallback is never saved as the full review.

**Commenting and reviewing**
- Click the `+` beside a diff line to comment. In split view, both columns have a gutter button. To comment on several lines, select them on one side within a patch hunk and click **Comment on lines…**, or Shift-click another line’s `+` to extend an open comment. Double-click selects a word by default. To open a comment with a double-click, turn on **Settings → File Diff → Double-click to comment**.
- Reply to, edit, delete and resolve GitHub conversations.
- To propose a replacement, start a GitHub comment on one or more lines of the new version and click **Suggest change**. Edit the prefilled code and add the draft to your review.
- Each comment goes either to GitHub, as a draft in your next review, or stays local. GitHub comments, replies, edits and review summaries open in **Rich text**. Apply a style as you type or format a selection. Use **Markdown** to edit the source and **Preview** to read the rendered draft. Formatting buttons stay visible while they fit, with the rest under **More**. In a rich list, Return starts another item, Return on an empty item ends the list, and Shift-Return continues the current item on a new line. Bold (⌘B), italic (⌘I), links (⌘K) and undo/redo (⌘Z/⇧⌘Z) also have shortcuts.
- Submit a GitHub review: comment, approve or request changes, with a summary.
- Local notes copy as Markdown, with the file, the lines and the code around them, ready to paste to an AI agent. **Send to Claude…** in the Review tab also lets you choose an active local Claude Code session by name, working folder, activity and session ID. The session list fills the available panel height, with search and send controls kept visible. Sessions are grouped by repository, with linked worktrees together and a count of matching sessions per heading. Search by name, repository, folder, Git branch, full path or session ID; use the arrow keys to select and Enter to send. Compact folder labels and Git branches keep repositories and worktrees readable, with full details on hover. Repository groups and branches load in the background after the session list appears. The list refreshes when opened, and the destination is rechecked before sending.
- Sent notes include a short explanation that they came from a user reviewing in Quarterdeck and that Quarterdeck cannot receive replies. Notes remain saved here. A submission is not a delivery acknowledgement: check Claude for receipt, questions or an approval prompt; its inbound permissions still apply. Requires an installed CLI supporting `claude agents --json` and a local inbox.

**Briefs written by Claude (experimental)**
- Claude builds pages through a private MCP connection. Invalid anchors or malformed widgets return to Claude for repair before publication. The saved brief stays readable while a replacement is written and remains available if it fails or is cancelled. A completed brief that fails to save offers **Retry save** without another Claude run, or **Discard**. Unsaved briefs are retained for this session while their reviewed commits remain current.
- Switch a pull request from **Diff** to **Brief** and click **Write brief**: Claude Code reads the change's diffs and walks you through it the way a colleague would: why it exists, the context you need, what changes and what stays the same, the order to read it in, how it works, and its limits. It explains; it does not review or judge the change.
- Before writing, say how well you know the area: **I know this code well** gives just the change, **I know the area roughly** explains the mechanisms it touches, and **I'm new to this area** starts with background on that part of the system, read from the surrounding code. Quarterdeck remembers the answer per repository until you quit.
- Briefs are kept: a ✦ in the sidebar marks pull requests that have one, and a brief says above it which commit it was written for, and warns with **Write again** when the pull request has moved on since.
- Something wrong or weird in a brief? **Report…** at the right of the line above it saves your feedback, the brief, and available publication evidence (including the MCP receipt and repair history). Reports made in the session that generated the brief also include the accepted raw artifact and transcript. Reports are written to `~/Library/Application Support/Quarterdeck/brief-reports/`. Hand the folder's `report.md` to a Claude Code session in this repository to improve the brief tool. Code in the brief is quoted from the diff itself, and every quote opens the diff at that line. A larger change reads like a short presentation: pages from the overview through each mechanism to what to check, with tabs to jump and buttons to the next page. Where a picture explains more, the brief draws diagrams: who owns what and where data goes (before and after, when the change reshapes it), state machines, and message sequences.
- Claude gets a shallow checkout of the pull request's head commit (Git LFS files left out), plus its diff, to follow callers, types and tests. It can only read files in that checkout, which is deleted when the brief is done; it cannot run or change anything. The code it reads is sent to Anthropic.
- Needs [Claude Code](https://claude.ai/code) installed and logged in, and `git` (Xcode's command line tools). Saved briefs survive relaunch and cache cleanup.
- **Window → Brief Gallery** shows every part a brief can have on sample text, without running Claude.

**Acting on a pull request**
- From the side panel's Actions tab: merge with any merge method the repository allows, as admin when you may bypass its rules, or through its merge queue. Merging a GitHub stack layer also merges the native layers below it. A branch match is used for reading and navigation only.
- The side panel's Checks tab shows how the CI checks of the shown pull request stand (in a stack, the shown layer's). It uses the latest run of each workflow, so superseded failures and cancellations do not count. It leads with a verdict and an outcome bar, lists what failed, what still runs and which required checks have not reported yet (GitHub's "Expected": the base branch requires them, but nothing has started them), then every check by workflow and called workflow, folded, with skipped ones hidden until asked for. It refreshes every 30 s while checks run, and a click on a check opens its page. When GitHub refuses check runs to the token (a fine-grained token without the Checks permission), the tab reads the GitHub Actions jobs instead, which needs read access to Actions and Commit statuses; required checks are then matched by name and other apps' checks are missing. When a merge conflict keeps GitHub from running the workflows (in a stack, often a layer further down), the tab says so and names the pull request to fix.
- Close a pull request, and delete its branch.

**Light on GitHub**
- Each GitHub account has its own local queue, drafts, and file cache. A previously verified credential opens that account’s cached data offline.
- A cached file set at the same pull request timestamp opens locally. When the timestamp changes, Quarterdeck checks the commits. Paged downloads check both commits again before saving. Expanding context fetches the selected file. The first nonempty **Contents** query can fetch every uncached changed file at the displayed revision. Later searches reuse cached blobs.

Line notes retain the original diff excerpt and commit pair. After a push or rebase, outdated drafts stay in the Review panel. Use **Copy draft** to recover the text, recreate the comment on the current line, and remove the outdated draft before submitting. Merges and GitHub reviews use the head commit displayed in the diff.

## Install

Download `Quarterdeck-<version>.dmg` from the [latest release](https://github.com/plucked/quarterdeck/releases/latest), open it and drag Quarterdeck into Applications. It needs a Mac with Apple silicon and macOS 13 or later; it is signed and notarized, so it opens like any other app.

**Signing in.** Quarterdeck checks `GITHUB_TOKEN`, the token saved in Keychain, then the [GitHub CLI](https://cli.github.com) login (`gh auth login`). To save a token in Keychain, open Settings → GitHub, paste a [personal access token](https://github.com/settings/tokens/new?scopes=repo,read:org&description=Quarterdeck) with the `repo` and `read:org` scopes and click Sign in: it is checked with GitHub, saved in your Keychain, and the app restarts with it.

**Updates.** Quarterdeck looks for a new version shortly after launch and every few hours, downloads and checks it in the background (it must be signed by the same developer and notarized), and installs it when you quit. To install it at once, click **Restart to update** in the top bar, or in Settings → Updates, which also shows what's new. Quarterdeck → Check for Updates… checks right away. Updates need the app to be in a folder you can write to, such as Applications.

## Run it from source

You need macOS, [Rust](https://rustup.rs) (stable; `rust-toolchain.toml` picks it), [Git LFS](https://git-lfs.com) for the intro movie (without it the app starts without the intro), and access to GitHub through either the [GitHub CLI](https://cli.github.com) or a token:

```bash
gh auth login              # or: export GITHUB_TOKEN=<a token with repo access>
cargo run
# Open immediately for testing, without the startup splash:
cargo run -- --no-splash
```

Quarterdeck uses `GITHUB_TOKEN` when it is set, then a token saved in Settings → GitHub, then the GitHub CLI's login (found on `PATH` or where Homebrew installs it). Without a credential, Quarterdeck opens a separate signed-out profile. Builds from source don't update themselves.

Use `--no-splash` for UI testing and visual checks: `cargo run -- --no-splash`, or `target/debug/Quarterdeck.app/Contents/MacOS/Quarterdeck --no-splash` for a development bundle. Launch with the flag before attaching UI automation, including after a restart; querying a stopped app can launch it without arguments. Keep only one development instance running. Omit the flag only when testing the startup movie or its fade transition.

### As a Mac app

```bash
packaging/macos/bundle.sh               # builds target/bundle/Quarterdeck.app (Apple silicon)
```

It is signed for this Mac only; releases are signed and notarized by the release script.

### Releasing

Use `$release` in Codex or `/release` in Claude Code. Both follow the [same release workflow](.claude/skills/release/SKILL.md) and accept an optional `patch`, `minor`, `major`, or exact version.

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
| ⌘1 / ⌘2 / ⌘3 | Mine / To review / Others |
| ⌘R | Refresh |
| ⌘F | Find in the current file’s diff |
| ] / [ | Next / previous file |
| v | Mark the current file viewed and go to the next |
| } / { | Next / previous stack layer |
| c | Layer ↔ combined stack review |
| ⌘B | Show or hide the side panel |
| ⌃⌘S | Show or hide the sidebar |
| ⇧⌘F | Toggle Focus Mode |
| ⌘, | Settings |

## Data

Quarterdeck keeps the queue, review notes, viewed marks, and preferences for each verified GitHub account in `~/Library/Application Support/Quarterdeck/accounts/<account-id>.sqlite`. Rotating a token for the same account opens the same database. Signing out opens a separate profile. Settings → GitHub shows cache usage and can clear fetched data.

`cargo run`, `devbox run app`, and the debug app bundle use `Quarterdeck Dev/accounts/`. An explicit `cargo run --release` uses the installed app’s data directory and can migrate its schema beyond the version an older release understands.

Earlier versions kept one `cache.sqlite` without an account identifier. Quarterdeck preserves that file and starts fresh account profiles. To recover a legacy database that you know belongs entirely to one account, quit Quarterdeck and back up both databases with SQLite’s backup command. Restore the legacy backup to that account’s profile only if you intend to replace its current local data. Reopening Quarterdeck migrates the restored notes and guides. Legacy line notes with no recorded revision appear as outdated.

## License

Quarterdeck's original code and assets are released under the [Unlicense](LICENSE). You can use, copy, modify, redistribute, and sell them for any purpose, with no attribution required.

Third-party dependencies and bundled assets retain their own licenses, including [JetBrains Mono](crates/app/assets/fonts/JetBrainsMono-OFL.txt), [Schibsted Grotesk](crates/app/assets/fonts/SchibstedGrotesk-OFL.txt), [Octicons](crates/app/assets/icons/LICENSE-octicons.txt), and [Material Icon Theme](crates/app/assets/icons/LICENSE-material-icon-theme.txt).
