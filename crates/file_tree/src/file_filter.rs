use std::collections::BTreeMap;

use crate::{FacetCount, FacetFilter, FileKind, FilteredFile};

/// Which changed files show, in the tree and the diff alike: a switch for
/// viewed files, and two rows of chips that narrow together, by kind and
/// by file type (a file passes when both rows admit it).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileFilter {
    pub hide_viewed: bool,
    pub kinds: FacetFilter<FileKind>,
    /// File types as `file_type_of` gives them: `.rs`, `Dockerfile`.
    pub file_types: FacetFilter<String>,
}

impl FileFilter {
    /// Whether `file` shows.
    pub fn shows(&self, file: &FilteredFile<'_>) -> bool {
        !(self.hide_viewed && file.is_viewed) && self.shows_ignoring_viewed(file)
    }

    /// Whether `file` passes the kind and type rows; a folder's viewed
    /// checkbox covers these files, viewed or not.
    pub fn shows_ignoring_viewed(&self, file: &FilteredFile<'_>) -> bool {
        self.kinds.admits(&file.kind()) && self.file_types.admits(file.file_type())
    }

    /// Whether the chip rows narrow anything (the viewed switch aside).
    pub fn has_chips_set(&self) -> bool {
        self.kinds.is_active() || self.file_types.is_active()
    }

    /// Clears both chip rows; the viewed switch stays.
    pub fn clear_chips(&mut self) {
        self.kinds = FacetFilter::default();
        self.file_types = FacetFilter::default();
    }

    /// Each kind present among `files` (Code always), with how many files
    /// it has and how many the viewed switch and the type row let through.
    pub fn kind_counts(&self, files: &[FilteredFile<'_>]) -> Vec<FacetCount<FileKind>> {
        FileKind::ALL
            .into_iter()
            .filter_map(|kind| {
                let of_kind = files.iter().filter(|file| file.kind() == kind);
                let total = of_kind.clone().count();
                let matching = of_kind
                    .filter(|file| self.passes_other_than_kind(file))
                    .count();
                (total > 0 || kind == FileKind::Code).then_some(FacetCount {
                    value: kind,
                    total,
                    matching,
                })
            })
            .collect()
    }

    /// Each file type among `files` in alphabetical order of its name
    /// without the leading dot, ignoring case (`.cs`, `Dockerfile`,
    /// `.gitignore`, `.rs`), with how many files it has and how many the
    /// viewed switch and the kind row let through.
    pub fn file_type_counts(&self, files: &[FilteredFile<'_>]) -> Vec<FacetCount<String>> {
        let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
        for file in files {
            let entry = counts.entry(file.file_type()).or_default();
            entry.0 += 1;
            entry.1 += usize::from(self.passes_other_than_type(file));
        }
        let mut counts: Vec<FacetCount<String>> = counts
            .into_iter()
            .map(|(value, (total, matching))| FacetCount {
                value: value.to_owned(),
                total,
                matching,
            })
            .collect();
        counts.sort_by_cached_key(|count| (sort_name(&count.value), count.value.clone()));
        counts
    }

    /// What narrows the files, in words, for the summary line and the
    /// empty diff: `Moved`, `.schema`, `not Tests`, `not viewed`.
    pub fn describe(&self) -> Vec<String> {
        let mut parts: Vec<String> = Vec::new();
        parts.extend(self.kinds.only.iter().map(|kind| kind.label().to_owned()));
        parts.extend(self.file_types.only.iter().cloned());
        parts.extend(
            self.kinds
                .hidden
                .iter()
                .map(|kind| format!("not {}", kind.label())),
        );
        parts.extend(
            self.file_types
                .hidden
                .iter()
                .map(|file_type| format!("not {file_type}")),
        );
        if self.hide_viewed {
            parts.push("not viewed".to_owned());
        }
        parts
    }

    /// Whether `file` passes everything but the kind row.
    fn passes_other_than_kind(&self, file: &FilteredFile<'_>) -> bool {
        !(self.hide_viewed && file.is_viewed) && self.file_types.admits(file.file_type())
    }

    /// Whether `file` passes everything but the type row.
    fn passes_other_than_type(&self, file: &FilteredFile<'_>) -> bool {
        !(self.hide_viewed && file.is_viewed) && self.kinds.admits(&file.kind())
    }
}

/// A file type as sorted: without its leading dot, in lower case.
fn sort_name(file_type: &str) -> String {
    file_type
        .strip_prefix('.')
        .unwrap_or(file_type)
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FileFacets;

    fn facets(path: &str, is_unchanged_move: bool) -> FileFacets {
        FileFacets::of(path, is_unchanged_move)
    }

    fn file(facets: &FileFacets) -> FilteredFile<'_> {
        FilteredFile {
            facets,
            is_viewed: false,
        }
    }

    /// Picking Moved shows every moved file, whatever its type.
    #[test]
    fn picking_a_kind_ignores_other_types() {
        let mut filter = FileFilter::default();
        filter.kinds.toggle_only(&FileKind::Moved);
        assert!(filter.shows(&file(&facets("contracts/a.schema", true))));
        assert!(!filter.shows(&file(&facets("src/a.rs", false))));
    }

    /// The rows narrow together: Moved and `.schema` together show moved schemas only.
    #[test]
    fn rows_narrow_together() {
        let mut filter = FileFilter::default();
        filter.kinds.toggle_only(&FileKind::Moved);
        filter.file_types.toggle_only(&".schema".to_owned());
        assert!(filter.shows(&file(&facets("a.schema", true))));
        assert!(!filter.shows(&file(&facets("a.cs", true))));
        assert_eq!(filter.describe(), ["Moved", ".schema"]);
    }

    /// Types sort by name without the dot, ignoring case, whatever their counts.
    #[test]
    fn file_types_sort_alphabetically() {
        let all_facets = [
            facets("a.rs", false),
            facets("b.rs", false),
            facets("Dockerfile", false),
            facets(".gitignore", false),
            facets("c.cs", false),
        ];
        let files: Vec<FilteredFile<'_>> = all_facets.iter().map(file).collect();
        let order: Vec<String> = FileFilter::default()
            .file_type_counts(&files)
            .into_iter()
            .map(|count| count.value)
            .collect();
        assert_eq!(order, [".cs", "Dockerfile", ".gitignore", ".rs"]);
    }

    /// Counts say what each chip would show given the other row.
    #[test]
    fn counts_follow_the_other_row() {
        let all_facets = [
            facets("a.schema", true),
            facets("b.schema", true),
            facets("c.cs", true),
            facets("d.cs", false),
        ];
        let files: Vec<FilteredFile<'_>> = all_facets.iter().map(file).collect();
        let mut filter = FileFilter::default();
        filter.kinds.toggle_only(&FileKind::Moved);
        let types = filter.file_type_counts(&files);
        assert_eq!(
            types,
            [
                FacetCount {
                    value: ".cs".to_owned(),
                    total: 2,
                    matching: 1
                },
                FacetCount {
                    value: ".schema".to_owned(),
                    total: 2,
                    matching: 2
                },
            ]
        );
        let kinds = filter.kind_counts(&files);
        assert_eq!(
            kinds
                .iter()
                .map(|count| (count.value, count.total))
                .collect::<Vec<_>>(),
            [(FileKind::Code, 1), (FileKind::Moved, 3)]
        );
    }

    /// The viewed switch hides viewed files on top of the chips.
    #[test]
    fn hide_viewed_hides_viewed_files() {
        let filter = FileFilter {
            hide_viewed: true,
            ..FileFilter::default()
        };
        let rust = facets("a.rs", false);
        let viewed = FilteredFile {
            is_viewed: true,
            ..file(&rust)
        };
        assert!(!filter.shows(&viewed));
        assert!(filter.shows_ignoring_viewed(&viewed));
    }
}
