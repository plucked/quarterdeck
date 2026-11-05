# packaging/macos

Turns the `quarterdeck` binary into `Quarterdeck.app` (`de.plucked.quarterdeck`): `bundle.sh` builds the release binary, writes `Info.plist` with the workspace's version, makes `AppIcon.icns` from the 1024 px `AppIcon.png`, copies the startup movie into `Contents/Resources`, and signs the bundle ad hoc for this Mac. With `--dev` it keeps a debug bundle in `target/debug/` up to date for bacon, so the app shows its icon and name while developing. It does not sign for distribution or notarize.
