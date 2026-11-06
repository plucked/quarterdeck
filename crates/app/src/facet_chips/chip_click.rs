use gpui::ClickEvent;

/// What a click on a filter chip asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipClick {
    /// A plain click: show only this (with other picked chips of its row).
    Pick,
    /// An option-click: hide this.
    Hide,
}

impl ChipClick {
    /// A plain click picks; an option-click hides.
    pub fn of(event: &ClickEvent) -> Self {
        if event.modifiers().alt {
            Self::Hide
        } else {
            Self::Pick
        }
    }
}
