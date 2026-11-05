use std::collections::BTreeSet;

use crate::file_category::{file_type_of, is_generated_path, is_test_path};
use crate::{FileGroup, FilteredFile};

/// Which changed files to hide, in the tree and the diff alike.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileFilter {
    pub hidden_groups: BTreeSet<FileGroup>,
    /// File types (as [`file_type_of`] gives them: `.rs`, `Dockerfile`)
    /// whose files are hidden.
    pub hidden_file_types: BTreeSet<String>,
}

impl FileFilter {
    /// Whether `file` shows.
    pub fn shows(&self, file: &FilteredFile<'_>) -> bool {
        let hidden_by_type = self.hidden_file_types.contains(&file_type_of(file.path));
        let hidden_by_group = self
            .hidden_groups
            .iter()
            .any(|group| is_in_group(file, *group));
        !(hidden_by_group || hidden_by_type)
    }

    pub fn hides(&self, group: FileGroup) -> bool {
        self.hidden_groups.contains(&group)
    }

    /// Hides `group`, or shows it again.
    pub fn toggle_group(&mut self, group: FileGroup) {
        if !self.hidden_groups.remove(&group) {
            self.hidden_groups.insert(group);
        }
    }

    /// Whether anything is hidden by this filter at all.
    pub fn is_active(&self) -> bool {
        !self.hidden_groups.is_empty() || !self.hidden_file_types.is_empty()
    }

    /// Hides every group, and every one of `file_types`, so they can be
    /// shown again one by one.
    pub fn hide_all(&mut self, file_types: impl IntoIterator<Item = String>) {
        self.hidden_groups.extend(FileGroup::ALL);
        self.hidden_file_types.extend(file_types);
    }

    /// Hides nothing.
    pub fn show_all(&mut self) {
        *self = Self::default();
    }
}

/// Whether `file` belongs to `group`.
fn is_in_group(file: &FilteredFile<'_>, group: FileGroup) -> bool {
    match group {
        FileGroup::Viewed => file.is_viewed,
        FileGroup::Tests => is_test_path(file.path),
        FileGroup::Generated => is_generated_path(file.path),
        FileGroup::UnchangedMoves => file.is_unchanged_move,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, is_viewed: bool) -> FilteredFile<'_> {
        FilteredFile {
            path,
            is_viewed,
            is_unchanged_move: false,
        }
    }

    /// Each kind of filter hides its files and nothing else.
    #[test]
    fn filters_hide_their_files() {
        let mut filter = FileFilter::default();
        assert!(filter.shows(&file("src/a_test.go", true)));

        filter.toggle_group(FileGroup::Viewed);
        assert!(!filter.shows(&file("src/a.rs", true)));
        assert!(filter.shows(&file("src/a.rs", false)));

        filter.toggle_group(FileGroup::Tests);
        assert!(!filter.shows(&file("src/a_test.go", false)));

        filter.hidden_file_types.insert(".md".into());
        assert!(!filter.shows(&file("README.md", false)));
        filter.hidden_file_types.insert("Dockerfile".into());
        assert!(!filter.shows(&file("web/Dockerfile", false)));
        assert!(filter.shows(&file("src/a.rs", false)));
        assert!(filter.is_active());
    }

    /// Unchanged moves hide alone; a moved file with changes still shows.
    #[test]
    fn hides_unchanged_moves() {
        let mut filter = FileFilter::default();
        filter.toggle_group(FileGroup::UnchangedMoves);
        let moved = FilteredFile {
            is_unchanged_move: true,
            ..file("src/moved.rs", false)
        };
        assert!(!filter.shows(&moved));
        assert!(filter.shows(&file("src/edited.rs", false)));
    }

    /// Hiding all hides every file; showing all hides none.
    #[test]
    fn hide_all_and_show_all() {
        let mut filter = FileFilter::default();
        filter.hide_all([".rs".to_owned()]);
        assert!(!filter.shows(&file("src/a.rs", false)));
        filter.show_all();
        assert!(!filter.is_active());
    }
}
