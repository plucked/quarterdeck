use gpui::SharedString;

use super::ChipClick;

/// A click on one chip of a [`FacetChipRow`](super::FacetChipRow): which
/// value it was on and whether it picks or hides it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChipRowClick {
    /// The clicked chip's value, as the row was given it.
    pub value: SharedString,
    /// Pick (a plain click) or hide (an option-click).
    pub click: ChipClick,
}
