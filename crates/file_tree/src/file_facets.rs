use crate::FileKind;
use crate::file_category::file_type_of;

/// What the filter sorts a changed file by: its one kind and its type.
/// Worked out from the path once per file (when the files load), since
/// telling tests and generated code apart reads the whole path, and the
/// chips count every file on each refresh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacets {
    pub kind: FileKind,
    /// The extension, or the name of a file without one.
    pub file_type: String,
}

impl FileFacets {
    /// The facets of the file at `path`; `is_unchanged_move` for a file
    /// moved or copied with its content unchanged.
    pub fn of(path: &str, is_unchanged_move: bool) -> Self {
        Self {
            kind: FileKind::of(path, is_unchanged_move),
            file_type: file_type_of(path),
        }
    }
}
