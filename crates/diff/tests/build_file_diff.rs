//! Building file diffs from GitHub-style patches, through the public API.

use qc_diff::{ChangeHighlight, DiffLineKind, DiffOptions, FileDiff, SplitRow, WhitespaceMode};

const PATCH: &str = "@@ -1,4 +1,4 @@ fn main() {\n let a = 1;\n-let b = 2;\n-let c = 3;\n+let b = 20;\n+let c = 3;\n+let d = 4;\n }\n";

fn options(whitespace: WhitespaceMode, highlight: ChangeHighlight) -> DiffOptions {
    DiffOptions {
        whitespace,
        highlight,
    }
}

/// The n-th removed line sits beside the n-th added line; extra additions stand alone.
#[test]
fn pairs_changes_side_by_side() {
    let diff = FileDiff::build(PATCH, DiffOptions::default());
    let rows = diff.hunks()[0].split_rows();
    assert_eq!(
        rows,
        [
            SplitRow {
                old: Some(0),
                new: Some(0)
            },
            SplitRow {
                old: Some(1),
                new: Some(3)
            },
            SplitRow {
                old: Some(2),
                new: Some(4)
            },
            SplitRow {
                old: None,
                new: Some(5)
            },
            SplitRow {
                old: Some(6),
                new: Some(6)
            },
        ]
    );
}

/// Word highlighting marks only the changed number in an edited line.
#[test]
fn marks_changed_words_in_pairs() {
    let diff = FileDiff::build(PATCH, options(WhitespaceMode::Show, ChangeHighlight::Words));
    let lines = &diff.hunks()[0].lines;
    assert_eq!(&lines[1].text[lines[1].changed_ranges[0].clone()], "2");
    assert_eq!(&lines[3].text[lines[3].changed_ranges[0].clone()], "20");
    // An unchanged pair (c = 3) and an unpaired addition carry no marks.
    assert_eq!(
        lines[4].changed_ranges,
        Vec::<std::ops::Range<usize>>::new()
    );
    assert_eq!(
        lines[5].changed_ranges,
        Vec::<std::ops::Range<usize>>::new()
    );
}

/// Line highlighting marks no ranges at all.
#[test]
fn line_highlighting_marks_no_ranges() {
    let diff = FileDiff::build(PATCH, options(WhitespaceMode::Show, ChangeHighlight::Lines));
    assert!(
        diff.hunks()[0]
            .lines
            .iter()
            .all(|line| line.changed_ranges.is_empty())
    );
}

/// Ignoring whitespace turns a re-indented line into context with both numbers.
#[test]
fn folds_whitespace_only_changes() {
    let patch = "@@ -3,2 +3,2 @@\n-  if x {\n-    go();\n+    if x {\n+    go_now();\n";
    let diff = FileDiff::build(
        patch,
        options(WhitespaceMode::Ignore, ChangeHighlight::Words),
    );
    let lines = &diff.hunks()[0].lines;
    let kinds: Vec<_> = lines.iter().map(|line| line.kind).collect();
    assert_eq!(
        kinds,
        [
            DiffLineKind::Context,
            DiffLineKind::Removed,
            DiffLineKind::Added
        ]
    );
    assert_eq!(
        (lines[0].old_number, lines[0].new_number),
        (Some(3), Some(3))
    );
    assert_eq!(lines[0].text, "    if x {");
}

/// A hunk of only whitespace changes disappears when whitespace is ignored.
#[test]
fn drops_whitespace_only_hunks() {
    let patch = "@@ -1 +1 @@\n-a  b\n+a b\n@@ -9 +9 @@\n-x\n+y\n";
    let ignored = FileDiff::build(
        patch,
        options(WhitespaceMode::Ignore, ChangeHighlight::Words),
    );
    assert_eq!(ignored.hunks().len(), 1);
    assert_eq!(ignored.changed_line_count(), 2);

    let shown = FileDiff::build(patch, options(WhitespaceMode::Show, ChangeHighlight::Words));
    assert_eq!(shown.hunks().len(), 2);
}

/// The gutter width comes from the largest number on either side.
#[test]
fn reports_max_line_number() {
    let diff = FileDiff::build("@@ -98,2 +120,2 @@\n a\n-b\n+c\n", DiffOptions::default());
    assert_eq!(diff.max_line_number(), 121);
}

/// An empty patch (binary file) has no hunks.
#[test]
fn empty_patch_has_no_hunks() {
    assert!(FileDiff::build("", DiffOptions::default()).is_empty());
}
