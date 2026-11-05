# packaging/macos

Turns the `quarterdeck` binary into `Quarterdeck.app` (`de.plucked.quarterdeck`): `bundle.sh` builds the release binary, writes `Info.plist` with the workspace's version, makes `AppIcon.icns` from the 1024 px `AppIcon.png`, and signs the bundle ad hoc for this Mac. It does not sign for distribution or notarize.
