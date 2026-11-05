/// What a click on a filter chip asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipClick {
    /// A plain click: show only this (with other picked chips of its row).
    Pick,
    /// An option-click: hide this.
    Hide,
}
