/// The boxes of a graph that share one band and one layer, in drawing order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphCell {
    pub band: usize,
    pub layer: usize,
    /// The boxes, by node index.
    pub nodes: Vec<usize>,
}
