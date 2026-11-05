# ui_state

Remembers what the UI showed so the next launch, typically a relaunch after a rebuild, opens the window in the same place with the same pull request and scroll position. It reads and writes that state through `qc-cache`'s `UiStateStore` and batches writes so scrolling does not hit the disk per event. It does not decide what a view restores when the things it points at are gone; each view does that.
