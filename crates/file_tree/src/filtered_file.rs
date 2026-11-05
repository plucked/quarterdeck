use crate::{FileFacets, FileKind};

/// What the filter looks at in one changed file: its facets and whether it
/// is marked viewed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilteredFile<'facets> {
    pub facets: &'facets FileFacets,
    pub is_viewed: bool,
}

impl FilteredFile<'_> {
    pub fn kind(&self) -> FileKind {
        self.facets.kind
    }

    /// The extension, or the name of a file without one.
    pub fn file_type(&self) -> &str {
        &self.facets.file_type
    }
}
