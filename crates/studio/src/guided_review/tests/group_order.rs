use crate::guided_review::DraftStatus;

use super::fixtures::{candidate, draft, order};

/// The complete lifecycle keeps an agent's file sequence even when inventory order differs.
#[test]
fn explicit_group_order_survives_recovery_and_finish() {
    let mut draft = draft(&["a-implementation.rs", "b-contract.rs", "c-tests.rs"]);
    let proposal = candidate(&[&["b-contract.rs", "a-implementation.rs", "c-tests.rs"]]);
    let accepted = draft.replace_bucket("behavior", 0, "write", proposal.clone());
    assert_eq!(accepted.status, DraftStatus::Accepted);
    assert_eq!(draft.bucket("behavior"), Some(&proposal));

    // Recovery and an idempotent retry cannot normalize the author's sequence.
    assert_eq!(
        draft.replace_bucket("behavior", 0, "write", proposal.clone()),
        accepted
    );
    let finished = draft.finish_guide(1, &order(&["behavior"]), "finish");
    assert_eq!(finished.status, DraftStatus::Ready);
    let buckets = draft
        .finished_buckets(finished.receipt.unwrap().as_str())
        .unwrap();
    assert_eq!(buckets[0].items[0].files, proposal.items[0].paths);
    assert_eq!(buckets[0].files, proposal.items[0].paths);
}

/// Explicit priority paths precede folder remainder, with duplicates keeping their first position.
#[test]
fn folder_remainder_preserves_frozen_inventory_order() {
    let mut draft = draft(&["assets/z.bin", "assets/a.bin", "format.md", "assets/m.bin"]);
    let proposal = candidate(&[&[
        "format.md",
        "assets/m.bin",
        "assets/",
        "assets/a.bin",
        "assets/",
    ]]);
    assert_eq!(
        draft.replace_bucket("assets", 0, "write", proposal).status,
        DraftStatus::Accepted
    );

    // Frozen inventory order is intentional even when it is not alphabetical.
    let receipt = draft
        .finish_guide(1, &order(&["assets"]), "finish")
        .receipt
        .unwrap();
    let buckets = draft.finished_buckets(receipt.as_str()).unwrap();
    assert_eq!(
        buckets[0].items[0].files,
        ["format.md", "assets/m.bin", "assets/z.bin", "assets/a.bin"]
    );
}

/// Identical membership in a different file sequence cannot reuse a finished receipt.
#[test]
fn receipt_binds_member_file_order() {
    let mut first = draft(&["a", "b"]);
    first.replace_bucket("group", 0, "write", candidate(&[&["a", "b"]]));
    let first_receipt = first
        .finish_guide(1, &order(&["group"]), "finish")
        .receipt
        .unwrap();

    // Keep run, revision, bucket and item identities equal; change only member order.
    let mut second = draft(&["a", "b"]);
    second.replace_bucket("group", 0, "write", candidate(&[&["b", "a"]]));
    let second_receipt = second
        .finish_guide(1, &order(&["group"]), "finish")
        .receipt
        .unwrap();
    assert_ne!(first_receipt, second_receipt);
    assert!(second.finished_buckets(first_receipt.as_str()).is_err());
}
