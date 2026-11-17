use crate::guided_review::{DraftStatus, GuideIssueCode, GuidedReviewDraft};

use super::fixtures::{candidate, draft, order};

/// Replays return the original result before stale-revision checks; changed input conflicts.
#[test]
fn replay_precedes_revision_and_payload_conflicts() {
    let mut draft = draft(&["a", "b"]);
    let first = draft.replace_bucket("core", 0, "create", candidate(&[&["a"]]));
    draft.replace_bucket("other", 1, "other", candidate(&[&["b"]]));
    assert_eq!(
        draft.replace_bucket("core", 0, "create", candidate(&[&["a"]])),
        first
    );
    assert_eq!(draft.draft().revision, 2);
    let conflict = draft.replace_bucket("core", 0, "create", candidate(&[&["b"]]));
    assert_eq!(conflict.issues[0].code, GuideIssueCode::OperationConflict);
    assert_eq!(draft.bucket("core").unwrap().items[0].paths, ["a"]);
}

/// Rejected attempts also replay exactly; repair content requires its own operation ID.
#[test]
fn invalid_attempts_are_idempotent() {
    let mut draft = draft(&["a"]);
    let rejected = draft.replace_bucket("core", 0, "attempt", candidate(&[&["missing"]]));
    draft.replace_bucket("core", 0, "repair", candidate(&[&["a"]]));
    assert_eq!(
        draft.replace_bucket("core", 0, "attempt", candidate(&[&["missing"]])),
        rejected
    );
    assert_eq!(
        draft
            .replace_bucket("core", 0, "attempt", candidate(&[&["a"]]))
            .issues[0]
            .code,
        GuideIssueCode::OperationConflict
    );
}

/// Two mutations based on the same revision have one winner; a later edit invalidates old validation.
#[test]
fn revision_conflicts_and_removal_preserve_accepted_state() {
    let mut draft = draft(&["a"]);
    assert_eq!(
        draft
            .replace_bucket("one", 0, "one", candidate(&[&["a"]]))
            .revision,
        1
    );
    assert_eq!(
        draft
            .replace_bucket("two", 0, "two", candidate(&[&["a"]]))
            .issues[0]
            .code,
        GuideIssueCode::RevisionConflict
    );
    assert_eq!(
        draft.validate_guide(1, &order(&["one"])).status,
        DraftStatus::Valid
    );
    assert_eq!(draft.remove_bucket("one", 1, "remove").revision, 2);
    assert_eq!(
        draft
            .finish_guide(1, &order(&["one"]), "stale-finish")
            .issues[0]
            .code,
        GuideIssueCode::RevisionConflict
    );
    assert_eq!(draft.draft().buckets, []);
    assert_eq!(
        draft.remove_bucket("one", 2, "already-gone").issues[0].code,
        GuideIssueCode::UnknownBucket
    );
    assert_eq!(draft.draft().revision, 2);
}

/// Input bytes, rather than bucket or attempt count, bound an oversized proposal.
#[test]
fn rejects_oversized_input_without_mutation() {
    let mut draft = draft(&["a"]);
    let mut oversized = candidate(&[&["a"]]);
    oversized.description = "x".repeat(1024 * 1024);
    let response = draft.replace_bucket("core", 0, "large", oversized);
    assert_eq!(response.issues[0].code, GuideIssueCode::ResourceLimit);
    assert_eq!(draft.draft().revision, 0);
    assert_eq!(draft.draft().buckets, []);
}

/// A small input with repeated broad folders stops expansion before allocating the full fan-out.
#[test]
fn expansion_budget_preserves_previous_complete_bucket() {
    let paths = (0..4000)
        .map(|index| format!("fixtures/{index}.bin"))
        .collect();
    let mut draft = GuidedReviewDraft::new("run".into(), "snapshot".into(), paths);
    let accepted = candidate(&[&["fixtures/"]]);
    assert_eq!(
        draft
            .replace_bucket("assets", 0, "initial", accepted.clone())
            .status,
        DraftStatus::Accepted
    );

    // This input is tiny, but unbounded expansion would construct 1.6 million memberships.
    let mut oversized_expansion = accepted.clone();
    oversized_expansion.items = vec![oversized_expansion.items[0].clone(); 400];
    let rejected = draft.replace_bucket("assets", 1, "expansion", oversized_expansion);
    assert_eq!(rejected.issues[0].code, GuideIssueCode::ResourceLimit);
    assert_eq!(draft.draft().revision, 1);
    assert_eq!(draft.bucket("assets"), Some(&accepted));
    assert_eq!(
        draft.validate_guide(1, &order(&["assets"])).status,
        DraftStatus::Valid
    );
}

/// Exhausting replay memory rejects further mutation while preserving every earlier operation.
#[test]
fn replay_capacity_never_evicts_successful_operations() {
    let mut draft = draft(&["a"]);
    let original = candidate(&[&["a"]]);
    let accepted = draft.replace_bucket("core", 0, "initial", original.clone());
    let mut invalid = original.clone();
    invalid.items[0].paths = (0..64)
        .map(|index| format!("unknown-{index}-{}", "x".repeat(80)))
        .collect();

    // Large bounded diagnostic samples fill the byte ledger without a fixed attempt policy.
    let exhausted = (0..512).any(|index| {
        let response =
            draft.replace_bucket("core", 1, &format!("attempt-{index}"), invalid.clone());
        response.issues[0].code == GuideIssueCode::ResourceLimit
    });
    assert!(exhausted);
    assert_eq!(draft.draft().revision, 1);
    assert_eq!(
        draft.replace_bucket("core", 0, "initial", original.clone()),
        accepted
    );
    assert_eq!(draft.bucket("core"), Some(&original));
}

/// Thousands of explicit exact paths fit in request memory without requiring folder selectors.
#[test]
fn large_explicit_inventory_fits_the_request_budget() {
    let paths: Vec<String> = (0..3803)
        .map(|index| format!("fixtures/explicit/{index}.json"))
        .collect();
    let mut draft = GuidedReviewDraft::new("run".into(), "snapshot".into(), paths.clone());
    let mut proposal = candidate(&[&["placeholder"]]);
    proposal.items[0].paths = paths;
    let response = draft.replace_bucket("fixtures", 0, "explicit", proposal);
    assert_eq!(response.status, DraftStatus::Accepted);
    assert_eq!(response.coverage.covered_paths, 3803);
}
