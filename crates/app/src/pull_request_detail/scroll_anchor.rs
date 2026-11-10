/// Where the list scrolls after its rows are rebuilt.
#[derive(Clone, Copy)]
pub enum ScrollAnchor {
    Top,
    /// The header of this file, which was at the top before.
    File(usize),
}
