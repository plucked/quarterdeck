# packaging/macos

Turns the `quarterdeck` binary into `Quarterdeck.app` (`de.plucked.quarterdeck`, Apple silicon only) and publishes it. `bundle.sh` builds the release binary, writes `Info.plist` with the workspace's version, makes `AppIcon.icns` from the 1024 px `AppIcon.png`, copies the startup movie into `Contents/Resources`, and signs the bundle ad hoc for this Mac. With `--dev` it keeps a debug bundle in `target/debug/` up to date for bacon, so the app shows its icon and name while developing.

`release.sh` makes a release from this Mac: it runs `bundle.sh`, signs the app with the Developer ID (hardened runtime), has Apple notarize the app and a disk image, writes `latest.json` (the manifest running apps read to update themselves) and publishes all three on GitHub releases. Its header lists the one-time setup. How a running app checks, verifies and installs a release is the `qd-update` crate's part.
