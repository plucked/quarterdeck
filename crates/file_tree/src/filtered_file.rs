use crate::FileKind;
use crate::file_category::file_type_of;

/// What the filter looks at in one changed file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilteredFile<'path> {
    pub path: &'path str,
    pub is_viewed: bool,
    /// Moved or copied with its content unchanged.
    pub is_unchanged_move: bool,
}

impl FilteredFile<'_> {
    pub fn kind(&self) -> FileKind {
        FileKind::of(self.path, self.is_unchanged_move)
    }

    /// The extension, or the name of a file without one.
    pub fn file_type(&self) -> String {
        file_type_of(self.path)
    }
}
