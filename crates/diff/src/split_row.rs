/// One row of a side-by-side view: indexes into its hunk's `lines` for the
/// old (left) and new (right) side. A context line fills both sides; a
/// removed line sits beside the added line that replaces it; an unpaired
/// change leaves the other side empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitRow {
    pub old: Option<usize>,
    pub new: Option<usize>,
}
