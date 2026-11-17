use crate::guided_review::{DraftStatus, GuideIssueCode};

use super::fixtures::{candidate, draft, order};

/// A rejected folder/exact overlap names both selectors and leaves the prior bucket intact.
#[test]
fn repairs_overlap_without_replacing_accepted_content() {
    let mut draft = draft(&["src/main.rs", "src/lib.rs", "tests.rs"]);
    let accepted = candidate(&[&["src/"], &["tests.rs"]]);
    assert_eq!(
        draft
            .replace_bucket("runtime", 0, "initial", accepted.clone())
            .status,
        DraftStatus::Accepted
    );

    // Both selectors exist individually, but their expanded files intersect across items.
    let rejected = draft.replace_bucket(
        "runtime",
        1,
        "bad-overlap",
        candidate(&[&["src/"], &["src/main.rs"], &["tests.rs"]]),
    );
    assert!(rejected.is_error());
    assert_eq!(rejected.revision, 1);
    let issue = &rejected.issues[0];
    assert_eq!(issue.code, GuideIssueCode::DuplicateFileInBucket);
    assert_eq!(issue.bucket_id.as_deref(), Some("runtime"));
    assert_eq!(issue.path.as_deref(), Some("src/main.rs"));
    assert_eq!(issue.item_indexes, [0, 1]);
    assert_eq!(issue.locations, ["/items/0/paths/0", "/items/1/paths/0"]);
    assert_eq!(issue.selectors, ["src/", "src/main.rs"]);
    assert_eq!(draft.bucket("runtime"), Some(&accepted));

    // A fresh operation repairs only this bucket in the same draft.
    let repaired = draft.replace_bucket(
        "runtime",
        1,
        "repair-overlap",
        candidate(&[&["src/main.rs"], &["src/lib.rs", "tests.rs"]]),
    );
    assert_eq!(repaired.status, DraftStatus::Accepted);
    assert_eq!(repaired.revision, 2);
    assert_eq!(
        draft.validate_guide(2, &order(&["runtime"])).status,
        DraftStatus::Valid
    );
}

/// Duplicate selectors inside an item deduplicate; duplicate membership across buckets is legal.
#[test]
fn retains_asymmetric_membership_rules() {
    let mut draft = draft(&["src/a", "src/b"]);
    assert!(
        !draft
            .replace_bucket("one", 0, "first", candidate(&[&["src/", "src/a", "src/"]]))
            .is_error()
    );
    assert_eq!(draft.draft().buckets[0].file_count, 2);
    assert!(
        !draft
            .replace_bucket("two", 1, "second", candidate(&[&["src/a"]]))
            .is_error()
    );
    assert_eq!(
        draft.validate_guide(2, &order(&["two", "one"])).status,
        DraftStatus::Valid
    );
}

/// Independent invalid guidance and unknown selectors are reported together, with a bounded sample.
#[test]
fn aggregates_actionable_diagnostics() {
    let mut draft = draft(&["a"]);
    let mut proposal = candidate(&[&["missing-one", "missing-two"], &["a"], &["a"]]);
    proposal.description.clear();
    proposal.items[0].label.clear();
    let rejected = draft.replace_bucket("core", 0, "bad", proposal);
    assert_eq!(rejected.issue_count, 5);
    assert_eq!(
        rejected
            .issues
            .iter()
            .filter(|issue| issue.code == GuideIssueCode::UnknownPath)
            .count(),
        2
    );
    assert_eq!(draft.draft().revision, 0);
    assert_eq!(draft.draft().buckets, []);
}
