use std::collections::BTreeMap;

use crate::file_category::extension_of;

/// How many changed files have one extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionCount {
    /// Lower case with its dot (`.rs`).
    pub extension: String,
    pub count: usize,
}

impl ExtensionCount {
    /// The extensions among `paths`, most files first, then by name. Files
    /// without an extension are not counted.
    pub fn of(paths: &[&str]) -> Vec<Self> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for extension in paths.iter().filter_map(|path| extension_of(path)) {
            *counts.entry(extension).or_default() += 1;
        }
        let mut counts: Vec<Self> = counts
            .into_iter()
            .map(|(extension, count)| Self { extension, count })
            .collect();
        counts.sort_by(|left, right| {
            right
                .count
                .cmp(&left.count)
                .then_with(|| left.extension.cmp(&right.extension))
        });
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The most common extension comes first; ties go by name.
    #[test]
    fn counts_extensions_most_first() {
        let counts = ExtensionCount::of(&["a.rs", "b.rs", "c.md", "d.cs", "Makefile"]);
        let summary: Vec<_> = counts
            .iter()
            .map(|count| (count.extension.as_str(), count.count))
            .collect();
        assert_eq!(summary, [(".rs", 2), (".cs", 1), (".md", 1)]);
    }
}
