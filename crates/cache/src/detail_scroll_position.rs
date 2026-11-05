/// Where the detail pane is scrolled to: the row at its top and how far
/// into that row, so restoring needs no row heights. The rows are the
/// app's virtualized detail rows (header, description, file rows).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DetailScrollPosition {
    pub row: usize,
    /// Pixels from the row's top down to the pane's top edge.
    pub offset_in_row: f32,
}
