//! Building file diffs from GitHub-style patches, through the public API.

use qd_diff::{
    ChangeHighlight, DiffLineKind, DiffOptions, FileDiff, LineOrigin, SplitRow, WhitespaceMode,
};

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
    assert_eq!(lines[0].origin, LineOrigin::FoldedReplacement);
    assert_eq!(lines[1].origin, LineOrigin::Patch);
}

/// Ignoring whitespace folds a re-indented block wrapped in a new `if`,
/// even when the raw patch split the block across a spurious context brace or blank line.
#[test]
fn folds_block_wrapped_in_if_across_spurious_context() {
    let patch = "\
@@ -324,19 +331,21 @@ void Tick() {
     if (active == null) {
         return;
     }
 
-    // Multi-agent
-    if (multi != null) {
-        if (hold) {
-            return;
+    if (!authorityLossImminent) {
+        // Multi-agent
+        if (multi != null) {
+            if (hold) {
+                return;
+            }
+        }
+
+        // Issuer destroyed
+        if (destroyed) {
+            abort();
         }
-    }
 
-    // Issuer destroyed
-    if (destroyed) {
-        abort();
-    }
-
-    if (exhausted) {
-        stop();
+        if (exhausted) {
+            stop();
+        }
     }
";
    let diff = FileDiff::build(
        patch,
        options(WhitespaceMode::Ignore, ChangeHighlight::Words),
    );
    let lines = &diff.hunks()[0].lines;
    let changed: Vec<_> = lines
        .iter()
        .filter(|line| line.kind != DiffLineKind::Context)
        .map(|line| {
            (
                line.kind,
                line.old_number,
                line.new_number,
                line.text.as_str(),
            )
        })
        .collect();
    assert_eq!(
        changed,
        [
            (
                DiffLineKind::Added,
                None,
                Some(335),
                "    if (!authorityLossImminent) {"
            ),
            (DiffLineKind::Added, None, Some(351), "    }"),
        ]
    );
    // Original unchanged context keeps Patch origin; re-paired lines keep FoldedReplacement.
    assert_eq!(lines[0].origin, LineOrigin::Patch);
    assert_eq!(lines[5].origin, LineOrigin::FoldedReplacement);
    assert_eq!(
        (lines[5].old_number, lines[5].new_number),
        (Some(328), Some(336))
    );
}

/// Ignoring whitespace folds an unwrapped block and marks only the removed wrapper lines.
#[test]
fn folds_unwrapped_block_and_marks_outer_closing_brace() {
    let patch = "\
@@ -1,7 +1,5 @@
 before();
-if (cond) {
-    if (a) {
-        foo();
-    }
+if (a) {
+    foo();
 }
 after();
";
    let diff = FileDiff::build(
        patch,
        options(WhitespaceMode::Ignore, ChangeHighlight::Words),
    );
    let lines = &diff.hunks()[0].lines;
    let changed: Vec<_> = lines
        .iter()
        .filter(|line| line.kind != DiffLineKind::Context)
        .map(|line| {
            (
                line.kind,
                line.old_number,
                line.new_number,
                line.text.as_str(),
            )
        })
        .collect();
    assert_eq!(
        changed,
        [
            (DiffLineKind::Removed, Some(2), None, "if (cond) {"),
            (DiffLineKind::Removed, Some(6), None, "}"),
        ]
    );
    // The trailing unchanged line stays genuine Patch context.
    assert_eq!(lines.last().unwrap().origin, LineOrigin::Patch);
}

/// Ignoring whitespace still pairs and highlights an edited line inside a re-indented block.
#[test]
fn highlights_edited_line_inside_wrapped_block() {
    let patch = "\
@@ -1,5 +1,7 @@
 before();
-a();
-foo(1);
-b();
+if (cond) {
+    a();
+    foo(2);
+    b();
+}
 after();
";
    let diff = FileDiff::build(
        patch,
        options(WhitespaceMode::Ignore, ChangeHighlight::Words),
    );
    let lines = &diff.hunks()[0].lines;
    let removed = lines
        .iter()
        .find(|line| line.kind == DiffLineKind::Removed)
        .unwrap();
    let added = lines
        .iter()
        .find(|line| line.kind == DiffLineKind::Added && line.text.contains("foo"))
        .unwrap();
    assert_eq!(removed.text, "foo(1);");
    assert_eq!(added.text, "    foo(2);");
    assert_eq!(&removed.text[removed.changed_ranges[0].clone()], "1");
    assert_eq!(&added.text[added.changed_ranges[0].clone()], "2");
}

/// Pure-addition hunks keep every original context line untouched when whitespace is ignored.
#[test]
fn pure_addition_hunk_preserves_original_context_origin() {
    let patch = "@@ -1,3 +1,5 @@\n a\n+b\n+c\n b\n d\n";
    let diff = FileDiff::build(
        patch,
        options(WhitespaceMode::Ignore, ChangeHighlight::Words),
    );
    let lines = &diff.hunks()[0].lines;
    assert_eq!(
        lines
            .iter()
            .map(|line| (line.kind, line.origin, line.text.as_str()))
            .collect::<Vec<_>>(),
        [
            (DiffLineKind::Context, LineOrigin::Patch, "a"),
            (DiffLineKind::Added, LineOrigin::Patch, "b"),
            (DiffLineKind::Added, LineOrigin::Patch, "c"),
            (DiffLineKind::Context, LineOrigin::Patch, "b"),
            (DiffLineKind::Context, LineOrigin::Patch, "d"),
        ]
    );
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
