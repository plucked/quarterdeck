//! Showing a file's unchanged lines around its hunks, through the public API.

use qd_diff::{
    ContextGap, DiffOptions, FileDiff, GapPlace, LineOrigin, RevealedLines, SourceLines,
};

/// Two hunks of a 30-line file: line 5 changed, a line added after line 20.
const PATCH: &str =
    "@@ -4,3 +4,3 @@ fn a\n n4\n-old\n+changed\n n6\n@@ -20,2 +20,3 @@\n n20\n+added\n n22\n";

/// The file after the change: line n reads `n{n}`, but for the changed and added lines.
fn source() -> SourceLines {
    let text: String = (1..=30)
        .map(|number| match number {
            5 => "changed\n".to_owned(),
            21 => "added\n".to_owned(),
            _ => format!("n{number}\n"),
        })
        .collect();
    SourceLines::parse(&text)
}

fn diff() -> FileDiff {
    FileDiff::build(PATCH, DiffOptions::default())
}

/// The gaps are the top of the file, between the hunks, and the end, each
/// with the shift from new to old line numbers.
#[test]
fn finds_the_gaps_around_hunks() {
    assert_eq!(
        diff().gaps(Some(30)),
        [
            ContextGap {
                place: GapPlace::AboveHunk(0),
                new_start: 1,
                new_end: Some(4),
                old_offset: 0
            },
            ContextGap {
                place: GapPlace::AboveHunk(1),
                new_start: 7,
                new_end: Some(20),
                old_offset: 0
            },
            ContextGap {
                place: GapPlace::End,
                new_start: 23,
                new_end: Some(31),
                old_offset: -1
            },
        ]
    );
    // Without the file's length the end gap stays open.
    assert_eq!(diff().gaps(None)[2].new_end, None);
}

/// Revealed lines join the hunks they touch; the rest stays hidden, and
/// headers are worked out again.
#[test]
fn reveals_lines_around_hunks() {
    let mut revealed = RevealedLines::default();
    revealed.reveal(1..4);
    revealed.reveal(7..9);
    revealed.reveal(23..31);
    assert!(diff().is_patch_of(&source()));
    let expanded = diff().with_context(&source(), &revealed).unwrap();

    let headers: Vec<&str> = expanded
        .hunks()
        .iter()
        .map(|hunk| hunk.header.as_str())
        .collect();
    assert_eq!(headers, ["@@ -1,8 +1,8 @@ fn a", "@@ -20,10 +20,11 @@"]);

    // Revealed lines come from the file, numbered on both sides.
    let last = expanded.hunks()[1].lines.last().unwrap();
    assert_eq!(
        (last.old_number, last.new_number, last.text.as_str()),
        (Some(29), Some(30), "n30")
    );
    assert_eq!(last.origin, LineOrigin::FileText);
    assert_eq!(expanded.hunks()[0].lines[3].origin, LineOrigin::Patch);
    // Nothing is left below the last hunk.
    assert_eq!(expanded.gaps(Some(30)).len(), 1);
}

/// Revealing a whole gap merges the hunks on either side into one.
#[test]
fn a_revealed_gap_merges_its_hunks() {
    let mut revealed = RevealedLines::default();
    revealed.reveal(7..20);
    let expanded = diff().with_context(&source(), &revealed).unwrap();
    assert_eq!(expanded.hunks().len(), 1);
    assert_eq!(expanded.hunks()[0].header, "@@ -4,18 +4,19 @@ fn a");
    assert_eq!(expanded.changed_line_count(), diff().changed_line_count());
}

/// A text that is not what the patch leads to is not used.
#[test]
fn refuses_a_text_the_patch_does_not_match() {
    let mut revealed = RevealedLines::default();
    revealed.reveal(1..4);
    let other = SourceLines::parse("a\nb\nc\nd\ne\nf\n");
    assert_eq!(diff().with_context(&other, &revealed), None);
}
