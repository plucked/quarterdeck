use qd_cache::OthersFilter;
use qd_file_tree::FacetFilter;

/// A chip row of the Others filter: the pull requests' labels or their authors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OthersFacet {
    Labels,
    Authors,
}

impl OthersFacet {
    /// The picked and hidden values of this row in `filter`.
    pub fn chips_of(self, filter: &OthersFilter) -> &FacetFilter<String> {
        match self {
            Self::Labels => &filter.labels,
            Self::Authors => &filter.authors,
        }
    }

    /// The picked and hidden values of this row in `filter`, to change them.
    pub fn chips_of_mut(self, filter: &mut OthersFilter) -> &mut FacetFilter<String> {
        match self {
            Self::Labels => &mut filter.labels,
            Self::Authors => &mut filter.authors,
        }
    }

    /// The row's label beside its chips.
    pub fn label(self) -> &'static str {
        match self {
            Self::Labels => "Labels",
            Self::Authors => "Authors",
        }
    }

    /// A short name of the row, for its chips' element ids.
    pub fn id_part(self) -> &'static str {
        match self {
            Self::Labels => "label",
            Self::Authors => "author",
        }
    }

    /// The row's place in the filter, for its more / less link's element id.
    pub fn index(self) -> usize {
        match self {
            Self::Labels => 0,
            Self::Authors => 1,
        }
    }
}
