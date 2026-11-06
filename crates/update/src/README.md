# qd-update source

Lets the installed app replace itself with a newer release. It reads the newest release's `latest.json` from GitHub, downloads the zipped app it names, checks the download's size and checksum, unpacks it beside the running bundle and accepts it only when Apple's code signing says it comes from the same developer and is notarized. Installing swaps the two bundles; relaunching starts the new one after the old process has exited.

It does not decide when to check or show anything: the app's `app_update` module schedules the work on the background executor and shows the status. It does not build releases either; `packaging/macos/release.sh` does, and writes the manifest this crate reads.
