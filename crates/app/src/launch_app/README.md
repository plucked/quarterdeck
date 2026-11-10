# Launch app

Starts Quarterdeck: opens the database and the services, starts gpui, registers the app's commands and fonts, and opens the main window where it was last placed, hidden behind the startup splash while its movie plays (or at once without it). When the database cannot be opened, it says why in a system alert, since an app started from Finder has no terminal. It does not draw any window itself; the main window, the splash and the brief gallery live in their own folders.
