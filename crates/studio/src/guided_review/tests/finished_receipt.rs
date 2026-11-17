use crate::guided_review::{DraftStatus, GuideIssueCode, GuidedReviewDraft};

use super::fixtures::{candidate, draft, order};

/// Finished output is immutable, receipt-bound and recoverable by the identical finish retry.
#[test]
fn finishing_seals_and_replays_exact_receipt() {
    let mut draft = draft(&["a"]);
    assert!(draft.finished_buckets("invented").is_err());
    let accepted = draft.replace_bucket("core", 0, "create", candidate(&[&["a"]]));
    let finished = draft.finish_guide(1, &order(&["core"]), "finish");
    assert_eq!(finished.status, DraftStatus::Ready);
    assert_eq!(draft.finish_guide(1, &order(&["core"]), "finish"), finished);
    assert_eq!(
        draft.replace_bucket("core", 0, "create", candidate(&[&["a"]])),
        accepted
    );
    assert_eq!(
        draft.remove_bucket("core", 1, "remove").issues[0].code,
        GuideIssueCode::SealedDraft
    );
    assert_eq!(
        draft
            .replace_bucket("core", 1, "replace", candidate(&[&["a"]]))
            .issues[0]
            .code,
        GuideIssueCode::SealedDraft
    );
    assert!(draft.finished_buckets("invented").is_err());
    let receipt = finished.receipt.unwrap();
    let mut output = draft.finished_buckets(receipt.as_str()).unwrap();
    output[0].files.clear();
    assert_eq!(
        draft.finished_buckets(receipt.as_str()).unwrap()[0].files,
        ["a"]
    );
}

/// Snapshot input ownership freezes folder expansion, and receipts cannot cross runs or snapshots.
#[test]
fn captured_inventory_and_receipt_identity_are_stable() {
    let mut inventory = vec!["src/a".to_owned()];
    let mut first = GuidedReviewDraft::new("first".into(), "head:base".into(), inventory.clone());
    inventory.push("src/new".into());
    first.replace_bucket("core", 0, "create", candidate(&[&["src/"]]));
    let first_finished = first.finish_guide(1, &order(&["core"]), "finish");
    let receipt = first_finished.receipt.unwrap();
    assert_eq!(
        first.finished_buckets(receipt.as_str()).unwrap()[0].files,
        ["src/a"]
    );

    for (run, snapshot) in [("second", "head:base"), ("first", "head:new-base")] {
        let mut other = GuidedReviewDraft::new(run.into(), snapshot.into(), vec!["src/a".into()]);
        other.replace_bucket("core", 0, "create", candidate(&[&["src/"]]));
        let other_finished = other.finish_guide(1, &order(&["core"]), "finish");
        assert_ne!(other_finished.receipt.unwrap(), receipt);
        assert!(other.finished_buckets(receipt.as_str()).is_err());
    }
}

/// Identical run/revision/snapshot identities cannot authorize different ordered guide content.
#[test]
fn receipt_binds_content_and_reading_order() {
    let mut first = draft(&["a", "b"]);
    first.replace_bucket("first", 0, "first", candidate(&[&["a"]]));
    first.replace_bucket("second", 1, "second", candidate(&[&["b"]]));
    let original = first
        .finish_guide(2, &order(&["first", "second"]), "finish")
        .receipt
        .unwrap();

    for reverse_order in [false, true] {
        let mut other = draft(&["a", "b"]);
        let mut content = candidate(&[&["a"]]);
        if !reverse_order {
            content.description = "Different reading guidance.".into();
        }
        other.replace_bucket("first", 0, "first", content);
        other.replace_bucket("second", 1, "second", candidate(&[&["b"]]));
        let mut chosen_order = order(&["first", "second"]);
        if reverse_order {
            chosen_order.reverse();
        }
        let receipt = other
            .finish_guide(2, &chosen_order, "finish")
            .receipt
            .unwrap();
        assert_ne!(receipt, original);
        assert!(other.finished_buckets(original.as_str()).is_err());
    }
}
