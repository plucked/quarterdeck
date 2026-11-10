//! Filter chips: rows of values to click (show only these) or ⌥-click
//! (hide these), each with how many items it would show. Shared by the
//! file tree's filter and the Others tab's repository filters, so both
//! look and behave alike.
//!
//! Files:
//! - `chip_click.rs`: `ChipClick`, a chip clicked to pick or (option) to hide.
//! - `chip_row_click.rs`: `ChipRowClick`, a click on a chip of a `FacetChipRow`: its value and `ChipClick`.
//! - `facet_chip.rs`: `FacetChip`, one chip (picked, hidden or open) with its count.
//! - `facet_chip_row.rs`: `FacetChipRow`, a labelled row of the chips its caller chose, closed by a more / less link.
//! - `collapsed_chip_row.rs`: `CollapsedChipRow`, which chips of a row show and its more / less link; the two ways to pick a collapsed row's chips (`first_values`, `most_common_values`).
//! - `facet_chip_rendering.rs`: a labelled row of chips, the "more" link, the hint on how chips work.

mod chip_click;
mod chip_row_click;
mod collapsed_chip_row;
mod facet_chip;
mod facet_chip_rendering;
mod facet_chip_row;

pub use chip_click::ChipClick;
pub use chip_row_click::ChipRowClick;
pub use collapsed_chip_row::{CollapsedChipRow, first_values, most_common_values};
pub use facet_chip::FacetChip;
pub use facet_chip_rendering::{CHIP_HINT, render_chip_row};
pub use facet_chip_row::FacetChipRow;
