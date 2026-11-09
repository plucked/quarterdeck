use std::path::{Path, PathBuf};

/// The folder an agent reviews in: `head/` holds the head commit's files
/// (Git LFS files as their pointers), `change.diff` the pull request's
/// diff. Removed when dropped, so it lives only as long as the run;
/// a folder left by a crash is removed at the next launch
/// (`clean_orphan_workspaces`).
#[derive(Debug)]
pub struct ReviewWorkspace {
    folder: PathBuf,
}

impl ReviewWorkspace {
    /// Takes ownership of `folder`: it is removed when this is dropped.
    pub(crate) fn owning(folder: PathBuf) -> Self {
        Self { folder }
    }

    pub fn path(&self) -> &Path {
        &self.folder
    }
}

impl Drop for ReviewWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}
