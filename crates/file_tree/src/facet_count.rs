/// One chip's numbers: how many files have the value at all, and how many
/// of them the other filters let through (what picking it would show).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetCount<T> {
    pub value: T,
    pub total: usize,
    pub matching: usize,
}
