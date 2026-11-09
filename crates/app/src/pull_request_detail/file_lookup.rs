use qd_diff::FileDiff;

use super::loaded_file::LoadedFile;
use crate::brief_view::BriefDiffs;

/// The index of the loaded file `path` names, by its path or, for a
/// rename, its previous path.
pub fn file_index_of(files: &[LoadedFile], path: &str) -> Option<usize> {
    files.iter().position(|loaded| {
        loaded.file.filename == path || loaded.file.previous_filename.as_deref() == Some(path)
    })
}

/// A brief's anchors quote the loaded files' shown diffs.
impl BriefDiffs for &[LoadedFile] {
    fn diff_of(&self, path: &str) -> Option<&FileDiff> {
        file_index_of(self, path).map(|file| &*self[file].diff)
    }
}
