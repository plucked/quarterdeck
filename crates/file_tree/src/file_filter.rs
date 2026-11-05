use std::collections::BTreeSet;

use crate::file_category::{extension_of, is_generated_path, is_test_path};

/// Which changed files to hide, in the tree and the diff alike.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileFilter {
    pub hide_viewed: bool,
    pub hide_tests: bool,
    pub hide_generated: bool,
    /// Extensions (lower case, with the dot, as [`extension_of`] gives them)
    /// whose files are hidden.
    pub hidden_extensions: BTreeSet<String>,
}

impl FileFilter {
    /// Whether the file at `path` shows; `is_viewed` is its viewed mark.
    pub fn shows(&self, path: &str, is_viewed: bool) -> bool {
        let hidden_by_extension =
            extension_of(path).is_some_and(|extension| self.hidden_extensions.contains(&extension));
        !(self.hide_viewed && is_viewed
            || self.hide_tests && is_test_path(path)
            || self.hide_generated && is_generated_path(path)
            || hidden_by_extension)
    }

    /// Whether anything is hidden by this filter at all.
    pub fn is_active(&self) -> bool {
        self.hide_viewed
            || self.hide_tests
            || self.hide_generated
            || !self.hidden_extensions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each kind of filter hides its files and nothing else.
    #[test]
    fn filters_hide_their_files() {
        let mut filter = FileFilter::default();
        assert!(filter.shows("src/a_test.go", true));

        filter.hide_viewed = true;
        assert!(!filter.shows("src/a.rs", true));
        assert!(filter.shows("src/a.rs", false));

        filter.hide_tests = true;
        assert!(!filter.shows("src/a_test.go", false));

        filter.hidden_extensions.insert(".md".into());
        assert!(!filter.shows("README.md", false));
        assert!(filter.shows("src/a.rs", false));
        assert!(filter.is_active());
    }
}
