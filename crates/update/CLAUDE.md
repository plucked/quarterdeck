# qd-update

| Module | Holds |
|---|---|
| `app_version` | `AppVersion` (`major.minor.patch`, `Ord` by number, `current()` from `CARGO_PKG_VERSION`) |
| `release_manifest` | `ReleaseManifest`: `latest.json` decoded and checked (zip on `github.com/plucked/quarterdeck/releases/download/`, SHA-256 hex) |
| `update_feed` / `download_release` | `UpdateFeed` (`ureq` agent): `latest_release()`, one GET of `releases/latest/download/latest.json`; `download_release` streams the zip to disk, hashing it, refusing more than `zip_size` bytes or a wrong digest |
| `update_downloads` | `update_download_directory()`: `~/Library/Caches/de.plucked.quarterdeck/updates` |
| `code_signature` | `RELEASE_REQUIREMENT` (designated requirement of a Developer ID app of team `97ZZEC8R43`, id `de.plucked.quarterdeck`); `verify_release_signature` (`codesign --verify --deep --strict -R` + `spctl --assess`), `is_release_signed` (both crate-private, used by `running_bundle`) |
| `running_bundle` / `staged_update` / `update_blocker` | `RunningBundle::locate()` (from `current_exe`, refusing `UpdateBlocker`s: not a bundle, not release-signed, translocated, folder not writable), `stage_release` (`ditto -x -k` into `.<name>.app-update` beside the bundle, signature + version checks) → `StagedUpdate`, `install` (bundle → `previous.app`, staged → bundle, rolled back on failure), `remove_leftovers` |
| `relaunch_after_exit` | detached `/bin/sh` that waits for this pid to exit, then `open`s the bundle (or runs the bare executable) |
| `update_error` | `UpdateError` |

- Trust comes from Apple code signing, not a key of our own: an update is installed only when its signature satisfies the team's Developer ID requirement and Gatekeeper accepts it (notarized). The manifest's SHA-256 only catches damaged downloads.
- Staging sits beside the bundle (same volume) so installing is two renames; the running process keeps running from the moved files.
- Blocking and UI-free; system tools are called by absolute path (`/usr/bin/ditto`, `codesign`, `plutil`, `/usr/sbin/spctl`). Tests never touch the network; the swap is tested in a temp folder.
