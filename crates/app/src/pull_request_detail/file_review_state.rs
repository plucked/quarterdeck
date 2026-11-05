use std::collections::{HashMap, HashSet};

use qd_cache::{PatchFingerprint, ViewedFile};

use super::loaded_file::LoadedFile;
use super::viewed_toggle::ViewedToggle;
use crate::diff_view::FileDiffState;

/// Which of the selected pull request's files are viewed and which are
/// collapsed, by file name. Viewed files and large files start collapsed.
#[derive(Debug, Default)]
pub struct FileReviewState {
    viewed: HashSet<String>,
    collapsed: HashSet<String>,
}

impl FileReviewState {
    /// The state for freshly loaded `files`: a saved mark counts only when
    /// its fingerprint matches the file's current patch.
    pub fn for_loaded(files: &[LoadedFile], viewed_marks: &[ViewedFile]) -> Self {
        // The marks by file name, so matching stays linear for 17k files.
        let marks: HashMap<&str, PatchFingerprint> = viewed_marks
            .iter()
            .map(|mark| (mark.filename.as_str(), mark.fingerprint))
            .collect();
        let viewed: HashSet<String> = files
            .iter()
            .filter(|loaded| marks.get(loaded.filename()) == Some(&loaded.fingerprint))
            .map(|loaded| loaded.filename().to_owned())
            .collect();
        let collapsed = files
            .iter()
            .filter(|loaded| loaded.is_large() || viewed.contains(loaded.filename()))
            .map(|loaded| loaded.filename().to_owned())
            .collect();
        Self { viewed, collapsed }
    }

    /// How `file` shows.
    pub fn state_of(&self, file: &LoadedFile) -> FileDiffState {
        FileDiffState {
            is_collapsed: self.collapsed.contains(file.filename()),
            is_viewed: self.viewed.contains(file.filename()),
            is_large: file.is_large(),
        }
    }

    pub fn viewed_count(&self) -> usize {
        self.viewed.len()
    }

    pub fn is_viewed(&self, filename: &str) -> bool {
        self.viewed.contains(filename)
    }

    /// Marks `file` viewed (collapsing it) or not viewed (expanding it), for
    /// a folder's checkbox; `None` when it already was.
    pub fn set_viewed(&mut self, file: &LoadedFile, is_viewed: bool) -> Option<ViewedToggle> {
        (self.is_viewed(file.filename()) != is_viewed).then(|| self.toggle_viewed(file))
    }

    /// Collapses an expanded file or expands a collapsed one.
    pub fn toggle_collapsed(&mut self, filename: &str) {
        if !self.collapsed.remove(filename) {
            self.collapsed.insert(filename.to_owned());
        }
    }

    /// Marks `file` viewed (collapsing it) or unmarks it (expanding it).
    pub fn toggle_viewed(&mut self, file: &LoadedFile) -> ViewedToggle {
        let filename = file.filename().to_owned();
        if self.viewed.remove(&filename) {
            self.collapsed.remove(&filename);
            ViewedToggle::Unmarked(filename)
        } else {
            self.viewed.insert(filename.clone());
            self.collapsed.insert(filename.clone());
            ViewedToggle::Marked(ViewedFile {
                filename,
                fingerprint: file.fingerprint,
            })
        }
    }

    pub fn collapse_all(&mut self, files: &[LoadedFile]) {
        self.collapsed = files
            .iter()
            .map(|loaded| loaded.filename().to_owned())
            .collect();
    }

    pub fn expand_all(&mut self) {
        self.collapsed.clear();
    }
}

#[cfg(test)]
mod tests {
    use qd_diff::DiffOptions;
    use qd_github::PullRequestFile;

    use super::*;

    fn loaded(filename: &str, patch: &str) -> LoadedFile {
        LoadedFile::build(
            PullRequestFile {
                filename: filename.into(),
                previous_filename: None,
                status: "modified".into(),
                additions: 1,
                deletions: 1,
                patch: Some(patch.into()),
            },
            DiffOptions::default(),
        )
    }

    /// A saved mark holds only while the patch is the one it was made for.
    #[test]
    fn viewed_mark_needs_matching_fingerprint() {
        let unchanged = loaded("a.rs", "@@ -1 +1 @@\n-a\n+b\n");
        let changed = loaded("b.rs", "@@ -1 +1 @@\n-c\n+e\n");
        let marks = [
            ViewedFile {
                filename: "a.rs".into(),
                fingerprint: unchanged.fingerprint,
            },
            ViewedFile {
                filename: "b.rs".into(),
                fingerprint: loaded("b.rs", "@@ -1 +1 @@\n-c\n+d\n").fingerprint,
            },
        ];
        let state = FileReviewState::for_loaded(&[unchanged.clone(), changed.clone()], &marks);
        assert!(state.state_of(&unchanged).is_viewed);
        assert!(state.state_of(&unchanged).is_collapsed);
        assert!(!state.state_of(&changed).is_viewed);
    }

    /// Marking viewed collapses the file; unmarking expands it again.
    #[test]
    fn toggling_viewed_collapses_and_expands() {
        let file = loaded("a.rs", "@@ -1 +1 @@\n-a\n+b\n");
        let mut state = FileReviewState::for_loaded(std::slice::from_ref(&file), &[]);

        let marked = state.toggle_viewed(&file);
        assert!(matches!(marked, ViewedToggle::Marked(ref mark) if mark.filename == "a.rs"));
        assert!(state.state_of(&file).is_collapsed);

        assert_eq!(
            state.toggle_viewed(&file),
            ViewedToggle::Unmarked("a.rs".into())
        );
        assert!(!state.state_of(&file).is_collapsed);
        assert_eq!(state.viewed_count(), 0);
    }

    /// Large files start collapsed; collapse all and expand all cover every file.
    #[test]
    fn large_files_start_collapsed() {
        let large_patch = format!("@@ -1,0 +1,500 @@\n{}", "+x\n".repeat(500));
        let large = loaded("big.json", &large_patch);
        let small = loaded("a.rs", "@@ -1 +1 @@\n-a\n+b\n");
        let files = [large.clone(), small.clone()];
        let mut state = FileReviewState::for_loaded(&files, &[]);
        assert!(state.state_of(&large).is_collapsed);
        assert!(!state.state_of(&small).is_collapsed);

        state.collapse_all(&files);
        assert!(state.state_of(&small).is_collapsed);
        state.expand_all();
        assert!(!state.state_of(&large).is_collapsed);
    }
}
