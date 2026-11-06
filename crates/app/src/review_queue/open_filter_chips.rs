use qd_file_tree::FacetCount;

use super::chip_marks::ChipMarks;

/// The chip rows of the Others filter open under a repository's header,
/// worked out when the list or a filter changes, so drawing only reads them.
pub struct OpenFilterChips {
    /// The label chips with their counts, most matching first.
    pub labels: Vec<FacetCount<String>>,
    /// The author chips with their counts, most matching first.
    pub authors: Vec<FacetCount<String>>,
    /// What each chip shows before its name.
    pub marks: ChipMarks,
}
