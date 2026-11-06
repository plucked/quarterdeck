---
name: release
description: Publish a new Quarterdeck release (bump the version, write the notes, build, notarize, publish to GitHub).
disable-model-invocation: true
argument-hint: "[patch | minor | major | X.Y.Z]"
---

# Release Quarterdeck

Publish the current `main` as a new release; running apps pick it up through their updater. Invoking this skill is the go-ahead: run every step without asking, and stop only where a step says so.

`packaging/macos/release.sh` does the build, signing, notarization and upload; its header documents what it checks. This skill picks the version, writes the notes and drives the script.

## Steps

1. **Start from a pushed `main`.** `git switch main`, `git pull --ff-only`. Stop and report when the tree has uncommitted changes or `main` differs from `origin/main` after the pull.
2. **Collect the range.** The last release is `git describe --tags --abbrev=0 --match 'v*'`; list `git log --no-merges --format=%s <tag>..HEAD`. With no commits since the tag, report that there is nothing to release and stop.
3. **Pick the version** from the tag's version: `$ARGUMENTS` when given (`patch`, `minor`, `major`, or an explicit `X.Y.Z` above the last release); otherwise `minor` when the range has a `feat:` commit, else `patch`.
4. **Write the notes** to `target/release-notes-<version>.md`: Markdown for the people who use the app, under `### New` and `### Fixed` (omit an empty section). One plain-language line per user-visible change, from the `feat:` and `fix:` commits; merge commits that describe one change. Leave out `chore:`, `refactor:`, `docs:`, `test:`, `style:` and anything about building or releasing the app itself. Done when every user-visible commit in the range is covered by a line.
5. **Bump and push.** Set `version` under `[workspace.package]` in `Cargo.toml`, run `cargo check -q --workspace` so `Cargo.lock` follows, commit both as `chore: bump the version to <version>`, `git push origin main`.
6. **Run the release** in the background (5–10 minutes: a build and two notarizations): `packaging/macos/release.sh --notes target/release-notes-<version>.md`. Wait for its completion notification.
7. **Verify the feed.** `curl -sL https://github.com/plucked/quarterdeck/releases/latest/download/latest.json` shows the new `version`, and its `zip_sha256` equals `shasum -a 256` of `target/release-artifacts/<version>/Quarterdeck-<version>.zip`. Done when both match.
8. **Report** the release URL (`https://github.com/plucked/quarterdeck/releases/tag/v<version>`), the version, and the notes.

## When the script fails

- **Before notarization** (a check, the build, signing): fix the cause and rerun step 6. The version commit stays; no tag exists yet.
- **During notarization:** `xcrun notarytool log <submission id> --keychain-profile quarterdeck-notary` says why; fix and rerun step 6.
- **At publishing**, with the artifacts already notarized in `target/release-artifacts/<version>/`: publish them without rebuilding, with gh's own login:
  ```bash
  env -u GITHUB_TOKEN -u GH_TOKEN gh release create v<version> <zip> <dmg> <latest.json> --repo plucked/quarterdeck --target "$(git rev-parse HEAD)" --title "Quarterdeck <version>" --notes-file target/release-notes-<version>.md --draft
  env -u GITHUB_TOKEN -u GH_TOKEN gh release edit v<version> --repo plucked/quarterdeck --draft=false --latest
  ```
  A draft left by the failed run shows in `gh release list`; delete it first (`gh release delete <tag> --yes`) only when it is a draft.
