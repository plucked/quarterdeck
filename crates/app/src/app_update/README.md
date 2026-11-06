# app_update

Keeps the installed app current. A global `AppUpdater` finds out at launch whether the running app can replace itself (a release-signed bundle in a writable folder), then checks the newest GitHub release a little after launch and every few hours. A newer version is downloaded, unpacked beside the bundle and verified in the background; it is installed when the app quits, or at once when the user restarts from the top bar or the Updates settings page.

The work itself (feed, download, signature check, the bundle swap, relaunching) lives in the `qd-update` crate and runs on the background executor. This module only schedules it and holds the status that the top bar and the settings window show.
