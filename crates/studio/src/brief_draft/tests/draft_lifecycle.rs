use serde_json::{Value, json};

use crate::brief_draft::BriefDraft;
use crate::guided_review::DraftStatus;

use super::fixtures::{draft, order, page, section, targets};

/// Replays precede stale revisions and canonical object key order does not change request identity.
#[test]
fn canonical_replay_and_repair_identity_are_stable() {
    let mut draft = draft();
    let candidate = page(vec![section()]);
    let accepted = draft.replace_page("first", 0, "create", candidate.clone());
    draft.set_lead(1, "lead", "The change now retries.".into());
    let reordered: Value = serde_json::from_str(&format!(
        r#"{{"widgets":{},"summary":"What the change does.","title":"Behavior"}}"#,
        candidate["widgets"]
    ))
    .unwrap();
    assert_eq!(
        draft.replace_page("first", 0, "create", reordered),
        accepted
    );
    let conflict = draft.replace_page("first", 0, "create", page(vec![]));
    assert_eq!(conflict.issues[0].code, "operation_conflict");
    assert_eq!(draft.draft().revision, 2);
    assert_eq!(
        draft.replace_page("second", 0, "stale", candidate).issues[0].code,
        "revision_conflict"
    );
}

/// Failed replacements replay their original diagnostics and cannot erase valid state.
#[test]
fn rejected_replacements_are_replayable() {
    let mut draft = draft();
    let invalid = page(vec![super::fixtures::excerpt(999)]);
    let rejected = draft.replace_page("page", 0, "attempt", invalid.clone());
    draft.replace_page("page", 0, "repair", page(vec![section()]));
    assert_eq!(draft.replace_page("page", 0, "attempt", invalid), rejected);
    assert_eq!(draft.remove_page("page", 1, "remove").revision, 2);
    assert!(draft.page("page").is_none());
}

/// Completion requires a lead and the exact ordered set of one to ten accepted pages.
#[test]
fn complete_order_and_existing_page_limit_are_authoritative() {
    let mut draft = draft();
    assert!(draft.validate_brief(0, &[]).is_error());
    draft.set_lead(0, "lead", "A complete lead.".into());
    let mut ids = Vec::new();
    for index in 0..11_u64 {
        let id = format!("page-{index}");
        draft.replace_page(&id, index + 1, &id, page(vec![section()]));
        ids.push(id);
    }
    assert!(
        draft
            .finish_brief(12, &ids, "too-many")
            .issues
            .iter()
            .any(|issue| issue.code == "invalid_page_count")
    );
    draft.remove_page("page-10", 12, "remove-extra");
    ids.pop();
    assert!(draft.validate_brief(13, &ids[..9]).is_error());
    let mut duplicate = ids.clone();
    duplicate[1] = duplicate[0].clone();
    assert!(draft.validate_brief(13, &duplicate).is_error());
    assert_eq!(draft.validate_brief(13, &ids).status, DraftStatus::Valid);
}

/// A sealed Brief yields exactly its full artifact, rejects mutation and cannot cross run identities.
#[test]
fn receipt_freezes_full_artifact_and_run_identity() {
    let mut first = draft();
    first.set_lead(0, "lead", "The full explanation.".into());
    let candidate = page(vec![section()]);
    first.replace_page("page", 1, "page", candidate.clone());
    let ready = first.finish_brief(2, &order(&["page"]), "finish");
    assert_eq!(first.finish_brief(2, &order(&["page"]), "finish"), ready);
    let receipt = ready.receipt.unwrap();
    assert_eq!(
        first.set_lead(2, "change", "Changed".into()).issues[0].code,
        "sealed_draft"
    );
    assert!(first.finished_studio("forged").is_err());
    let mut finished = first.finished_studio(&receipt).unwrap();
    assert_eq!(
        finished.artifact,
        json!({"lead":"The full explanation.","pages":[candidate.clone()]})
    );
    finished.studio.pages.clear();
    assert_eq!(
        first.finished_studio(&receipt).unwrap().studio.pages.len(),
        1
    );

    for (run_id, snapshot_id) in [("other", "head:base"), ("run", "head:new-base")] {
        let mut other = BriefDraft::new(run_id.into(), snapshot_id.into(), targets());
        other.set_lead(0, "lead", "The full explanation.".into());
        other.replace_page("page", 1, "page", candidate.clone());
        let other_receipt = other
            .finish_brief(2, &order(&["page"]), "finish")
            .receipt
            .unwrap();
        assert_ne!(other_receipt, receipt);
        assert!(other.finished_studio(&receipt).is_err());
    }
}

/// Frozen targets cannot gain citable files when the host later adds another diff.
#[test]
fn anchor_inventory_is_owned_and_frozen() {
    let mut original = targets();
    let mut draft = BriefDraft::new("run".into(), "snapshot".into(), original.clone());
    original.add_file(
        "new.rs",
        None,
        &qd_diff::FileDiff::build("@@ -1 +1 @@\n-old\n+new", qd_diff::DiffOptions::default()),
    );
    let mut excerpt = super::fixtures::excerpt(10);
    excerpt["anchor"]["path"] = json!("new.rs");
    let response = draft.replace_page("new", 0, "new", page(vec![excerpt]));
    assert!(
        response
            .issues
            .iter()
            .any(|issue| issue.code == "invalid_anchor")
    );
    assert!(
        !draft
            .anchor_paths(0, 256)
            .paths
            .iter()
            .any(|path| path == "new.rs")
    );
}

/// Oversized text is a resource rejection, never an accepted page that cannot be recovered.
#[test]
fn bounds_wire_content_before_accepting_it() {
    let mut draft = draft();
    let mut candidate = page(vec![section()]);
    candidate["widgets"][0]["body"] = json!("x".repeat(900 * 1024));
    let response = draft.replace_page("large", 0, "large", candidate);
    assert_eq!(response.issues[0].code, "resource_limit");
    assert_eq!(draft.draft().revision, 0);
}

/// Earlier validation cannot finish a later revision, and receipts bind ordered content.
#[test]
fn edits_invalidate_validation_and_receipts_bind_order() {
    let mut first = draft();
    first.set_lead(0, "lead", "The explanation.".into());
    first.replace_page("one", 1, "one", page(vec![section()]));
    assert_eq!(
        first.validate_brief(2, &order(&["one"])).status,
        DraftStatus::Valid
    );
    first.replace_page("two", 2, "two", page(vec![section()]));
    assert_eq!(
        first
            .finish_brief(2, &order(&["one"]), "old-validation")
            .issues[0]
            .code,
        "revision_conflict"
    );
    let receipt = first
        .finish_brief(3, &order(&["one", "two"]), "finish")
        .receipt
        .unwrap();

    let mut reordered = draft();
    reordered.set_lead(0, "lead", "The explanation.".into());
    reordered.replace_page("one", 1, "one", page(vec![section()]));
    reordered.replace_page("two", 2, "two", page(vec![section()]));
    assert_ne!(
        reordered
            .finish_brief(3, &order(&["two", "one"]), "finish")
            .receipt
            .unwrap(),
        receipt
    );
}

/// A full replay ledger rejects further changes while retaining earlier success and diagnostics.
#[test]
fn replay_memory_never_evicts_prior_operations() {
    let mut draft = draft();
    let original = page(vec![section()]);
    let accepted = draft.replace_page("page", 0, "first", original.clone());
    let invalid = page(vec![json!({"kind":"unknown"}); 64]);
    let exhausted = (0..512).any(|index| {
        let result = draft.replace_page("page", 1, &format!("rejected-{index}"), invalid.clone());
        result.issues[0].code == "resource_limit"
    });
    assert!(exhausted);
    assert_eq!(draft.draft().revision, 1);
    assert_eq!(
        draft.replace_page("page", 0, "first", original.clone()),
        accepted
    );
    assert_eq!(draft.page("page"), Some(&original));
}
