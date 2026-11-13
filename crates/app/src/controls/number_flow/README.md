# NumberFlow

The native GPUI equivalent of [Number Flow's value transitions](https://number-flow.barvian.me/#basic-usage). It accepts an already formatted string so existing signs, leading zeros, units, grouping, precision and surrounding words stay intact. The first value is immediate; later changes roll the digits for 450 ms with a quintic ease-out. New and removed columns fade, and surviving columns move to their new positions. Each number has its own trend, with integer columns aligned to units and fractional columns to the decimal point.

```rust
div().text_xs().child(NumberFlow::new("viewed-count", format!("{viewed} of {total} viewed")))
```

Use a stable identity for the display, never its changing value. `NumberFlow::label(text)` uses its caller's source location for ordinary labels. Helpers forwarding that constructor use `#[track_caller]`; repeated items need a keyed parent (repository, file, check, comment, or row). Two numeric children in a shared helper need separate IDs, as the old/new diff gutters do.

The control inherits GPUI's typography, color, alignment, wrapping and ellipsis, with tabular digits. The final text owns layout; animation affects painting. The paint mask includes outgoing glyphs while a label shrinks, within its ancestors' clipping bounds. Interruptions continue from the visible wheel positions. It requests frames only while an on-screen value is transitioning, and keeps no timer or global cache. macOS Reduce Motion immediately settles the display to the final value.

`transition.rs` contains the tested, renderer-independent place-value and interruption model. `mod.rs` shapes and paints the animated label. The interactive preview runs with `cargo run -p quarterdeck --example number_flow`; it exercises small counts, carries, decreases, zero, decimals, percentages, right alignment, wrapping, truncation and both themes without opening the user's cache or connecting to GitHub.
