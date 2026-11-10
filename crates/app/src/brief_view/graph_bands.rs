/// The bands a graph's boxes go in: one per boundary, plus one for the
/// boxes in none, in the order the bands' first boxes are listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphBands {
    /// Each node's band, by node index.
    pub of_node: Vec<usize>,
    /// Each band's boundary (group index), by band; `None` for the band of
    /// boxes in no boundary.
    pub groups: Vec<Option<usize>>,
}
