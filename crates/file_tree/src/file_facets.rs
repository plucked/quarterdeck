use crate::file_category::file_type_of;
use crate::{FileIcon, FileKind};

/// What the filter sorts a changed file by: its one kind and its type, and
/// the icon the tree shows for it. Worked out from the path once per file
/// (when the files load), since telling tests and generated code apart
/// reads the whole path, the chips count every file on each refresh, and
/// the tree draws the visible rows' icons every frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacets {
    pub kind: FileKind,
    /// The extension, or the name of a file without one.
    pub file_type: String,
    pub icon: FileIcon,
}

impl FileFacets {
    /// The facets of the file at `path`; `is_unchanged_move` for a file
    /// moved or copied with its content unchanged.
    pub fn of(path: &str, is_unchanged_move: bool) -> Self {
        Self {
            kind: FileKind::of(path, is_unchanged_move),
            file_type: file_type_of(path),
            icon: FileIcon::of(path),
        }
    }
}
