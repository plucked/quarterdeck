/// What the change did to a part of a diagram. A diagram with added or
/// removed parts is drawn twice, before and after, at the same positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramChange {
    /// There before and after the change.
    Same,
    /// Only after the change.
    Added,
    /// Only before the change.
    Removed,
}

impl DiagramChange {
    /// Whether the part shows in the before picture.
    pub fn is_before(self) -> bool {
        matches!(self, Self::Same | Self::Removed)
    }

    /// Whether the part shows in the after picture.
    pub fn is_after(self) -> bool {
        matches!(self, Self::Same | Self::Added)
    }
}
