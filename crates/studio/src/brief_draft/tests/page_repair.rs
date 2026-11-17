use serde_json::json;

use crate::guided_review::DraftStatus;

use super::fixtures::{draft, excerpt, order, page, section};

/// An invalid anchor returns its exact pointer/ranges and preserves the earlier accepted page.
#[test]
fn repairs_anchor_in_the_same_draft_without_losing_content() {
    let mut draft = draft();
    draft.set_lead(0, "lead", "The caller retries transient failures.".into());
    let original = page(vec![section()]);
    draft.replace_page("behavior", 1, "initial", original.clone());
    let rejected = draft.replace_page("behavior", 2, "bad-anchor", page(vec![excerpt(99)]));
    assert!(rejected.is_error());
    assert_eq!(rejected.revision, 2);
    let issue = rejected
        .issues
        .iter()
        .find(|issue| issue.code == "invalid_anchor")
        .unwrap();
    assert_eq!(issue.page_id.as_deref(), Some("behavior"));
    assert_eq!(issue.widget_index, Some(0));
    assert_eq!(issue.location, "/widgets/0/anchor");
    assert_eq!(issue.path.as_deref(), Some("src/lib.rs"));
    assert_eq!(
        issue.allowed_ranges,
        std::iter::once(10..=13).collect::<Vec<_>>()
    );
    assert_eq!(draft.page("behavior"), Some(&original));

    let repaired = page(vec![excerpt(13), section()]);
    assert_eq!(
        draft
            .replace_page("behavior", 2, "repair", repaired.clone())
            .revision,
        3
    );
    let result = draft.finish_brief(3, &order(&["behavior"]), "finish");
    let finished = draft
        .finished_studio(result.receipt.as_deref().unwrap())
        .unwrap();
    assert_eq!(finished.artifact["pages"][0], repaired);
    assert_eq!(finished.studio.pages[0].widgets.len(), 2);
    let parsed = crate::parse_studio(finished.artifact, &super::fixtures::targets()).unwrap();
    assert_eq!(parsed.studio, finished.studio);
    assert_eq!(parsed.rejected, []);
}

/// Independent malformed diagrams and table widths reject the whole page with widget pointers.
#[test]
fn reports_table_and_graph_errors_together() {
    let mut draft = draft();
    let original = page(vec![section()]);
    draft.replace_page("behavior", 0, "initial", original.clone());
    let node = |id| json!({"id":id,"label":id,"detail":"","role":"owner","group":null,"anchor":null,"change":"same"});
    let malformed = page(vec![
        json!({"kind":"table","title":"State","columns":["Before","After"],"rows":[["one"]]}),
        json!({"kind":"graph","title":"Calls","groups":[],"nodes":[node("one"),node("two")],"edges":[{"from":"one","to":"missing","label":"calls","style":"control","change":"same"}]}),
    ]);
    let rejected = draft.replace_page("behavior", 1, "bad-widgets", malformed);
    assert!(
        rejected
            .issues
            .iter()
            .any(|issue| issue.location == "/widgets/0" && issue.message.contains("cells"))
    );
    assert!(
        rejected
            .issues
            .iter()
            .any(|issue| issue.location == "/widgets/1" && issue.message.contains("unknown node"))
    );
    assert_eq!(draft.page("behavior"), Some(&original));
    assert_eq!(draft.draft().revision, 1);
}

/// Strict authoring refuses dropped rows, empty scope entries and unknown fields.
#[test]
fn rejects_every_lossy_legacy_repair() {
    let mut draft = draft();
    let table =
        json!({"kind":"table","title":"Rows","columns":["Value"],"rows":vec![vec!["one"];41]});
    let scope = json!({"kind":"scope","changes":["Valid", "  "],"unchanged":[],"mechanical":[]});
    let mut extra = section();
    extra["unexpected"] = json!("would otherwise disappear");
    let proposal = page(vec![table, scope, extra]);
    let result = draft.replace_page("lossy", 0, "lossy", proposal.clone());
    for code in ["truncated_table", "discarded_scope_item", "unknown_field"] {
        assert!(
            result.issues.iter().any(|issue| issue.code == code),
            "missing {code}"
        );
    }
    assert!(draft.page("lossy").is_none());
    let legacy = crate::parse_studio(
        json!({"lead":"Lead","pages":[proposal]}),
        &super::fixtures::targets(),
    )
    .unwrap();
    assert_eq!(legacy.studio.pages[0].widgets.len(), 3);
}

/// Rename aliases and tour start-only semantics remain unchanged in strict page authoring.
#[test]
fn retains_existing_anchor_semantics() {
    let mut draft = draft();
    let tour = json!({"kind":"tour","title":"Read order","steps":[
        {"title":"Start","explanation":"Follow entry","weight":"core","anchor":{"path":"src/old.rs","side":"base","start_line":10,"end_line":999}},
        {"title":"Then","explanation":"Read the continuation","weight":"supporting","anchor":null}
    ]});
    let result = draft.replace_page("tour", 0, "tour", page(vec![tour]));
    assert_eq!(result.status, DraftStatus::Accepted);
    assert_eq!(
        draft
            .anchor_ranges("src/old.rs", crate::CodeSide::Base)
            .unwrap(),
        std::iter::once(10..=12).collect::<Vec<_>>()
    );
}

/// Invalid fields are independently diagnosed while response evidence stays bounded.
#[test]
fn bounds_many_page_diagnostics() {
    let mut draft = draft();
    let result = draft.replace_page(
        "invalid",
        0,
        "invalid",
        page(vec![json!({"kind":"unknown"}); 200]),
    );
    assert!(result.issue_count >= 200);
    assert!(result.issues.len() <= 64);
    assert!(
        result
            .issues
            .iter()
            .map(super::super::BriefIssue::storage_bytes)
            .sum::<usize>()
            <= 64 * 1024
    );
    assert_eq!(result.revision, 0);
}
