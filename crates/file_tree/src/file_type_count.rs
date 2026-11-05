use std::collections::BTreeMap;

use crate::file_category::file_type_of;

/// How many changed files have one file type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTypeCount {
    /// As [`file_type_of`] gives it: `.rs`, or a name like `Dockerfile`.
    pub file_type: String,
    pub count: usize,
}

impl FileTypeCount {
    /// The file types among `paths`, most files first, then by name.
    pub fn of(paths: &[&str]) -> Vec<Self> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for file_type in paths.iter().map(|path| file_type_of(path)) {
            *counts.entry(file_type).or_default() += 1;
        }
        let mut counts: Vec<Self> = counts
            .into_iter()
            .map(|(file_type, count)| Self { file_type, count })
            .collect();
        counts.sort_by(|left, right| {
            right
                .count
                .cmp(&left.count)
                .then_with(|| left.file_type.cmp(&right.file_type))
        });
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The most common type comes first; ties go by name; names without an extension count as their own type.
    #[test]
    fn counts_file_types_most_first() {
        let counts = FileTypeCount::of(&[
            "a.rs",
            "b.rs",
            "c.md",
            "Dockerfile",
            "web/Dockerfile",
            ".gitignore",
        ]);
        let summary: Vec<_> = counts
            .iter()
            .map(|count| (count.file_type.as_str(), count.count))
            .collect();
        assert_eq!(
            summary,
            [(".rs", 2), ("Dockerfile", 2), (".gitignore", 1), (".md", 1)]
        );
    }
}
