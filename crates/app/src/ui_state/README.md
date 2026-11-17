# ui_state

Remembers what the UI showed so the next launch, typically a relaunch after a rebuild, opens the window in the same place with the same pull request and scroll position. It reads and writes that state through `qd-cache`'s `UiStateStore` and batches writes so scrolling does not hit the disk per event. It does not decide what a view restores when the things it points at are gone; each view does that.

Review notes and summaries stay visible while SQLite writes are pending. The recorder retries failed writes in order and shows the error in the Review panel. Quitting drains pending and in-flight changes before the app exits.
