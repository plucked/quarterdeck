use std::sync::Arc;

use qd_diff::{DiffOptions, FileDiff};
use qd_studio::{
    CodeAccess, CodeSide, FileListing, ReaderFamiliarity, STUDIO_SCHEMA, StepWeight, StudioRequest,
    StudioRequestFile, StudioWidget, TableRole, parse_studio, studio_prompt,
};
use serde_json::json;

/// The request of a one-file pull request whose hunk shows base 10–12, head 10–13.
fn request() -> StudioRequest {
    let diff = FileDiff::build(
        "@@ -10,3 +10,4 @@\n a\n-b\n+c\n+d\n e",
        DiffOptions::default(),
    );
    StudioRequest {
        repository: "plucked/quarterdeck".into(),
        subject: "Pull request #7: Keep the whole-write flag".into(),
        author: "octocat".into(),
        description: String::new(),
        files: vec![StudioRequestFile {
            path: "src/map.rs".into(),
            previous_path: None,
            status: "modified".into(),
            additions: 2,
            deletions: 1,
            listing: FileListing::Diff(Arc::new(diff)),
        }],
        code: CodeAccess::DiffOnly("not asked".into()),
        reader: ReaderFamiliarity::Overview,
        layers: Vec::new(),
    }
}

fn anchor(side: &str, start: i64, end: i64) -> serde_json::Value {
    json!({ "path": "src/map.rs", "side": side, "start_line": start, "end_line": end })
}

/// Widgets with anchors in the diff are kept in order; the others are listed as rejected.
#[test]
fn keeps_anchored_widgets_and_rejects_the_rest() {
    let targets = studio_prompt(&request()).targets;
    let output = json!({
        "lead": "  Map writes keep the whole-write flag of maps they leave out.  ",
        "pages": [{ "title": " Overview ", "summary": "What changes.", "widgets": [
            { "kind": "section", "heading": "Clear tracking only for written fields", "body": "Text." },
            { "kind": "code_compare", "caption": "The guard", "before": anchor("base", 11, 11), "after": anchor("head", 11, 12) },
            { "kind": "code_excerpt", "caption": "Outside", "anchor": anchor("head", 40, 42) },
            { "kind": "code_compare", "caption": "Swapped", "before": anchor("head", 11, 11), "after": anchor("base", 11, 11) },
            { "kind": "trace", "title": "Failing case", "columns": ["Action", "Before", "After"], "rows": [["a", "b", "c"]] },
            { "kind": "table", "title": "Ragged", "columns": ["A", "B"], "rows": [["only one"]] },
            { "kind": "scope", "changes": [" Writes keep the flag ", ""], "unchanged": ["Reads"], "mechanical": [] },
            { "kind": "tour", "title": "Reading order", "steps": [
                { "title": "The guard", "explanation": "Decides.", "weight": "core", "anchor": anchor("head", 12, 400) },
                { "title": "Tests", "explanation": "Pin it.", "weight": "supporting", "anchor": null } ] },
            { "kind": "tour", "title": "Lost", "steps": [
                { "title": "Fine", "explanation": "E.", "weight": "core", "anchor": null },
                { "title": "Off", "explanation": "E.", "weight": "mechanical", "anchor": anchor("head", 90, 91) } ] },
            { "kind": "scope", "changes": [" "], "unchanged": [], "mechanical": [] }
        ] }]
    });

    let checked = parse_studio(output, &targets).expect("matches the schema");

    // The lead is trimmed; five widgets pass, in their order.
    assert_eq!(
        checked.studio.lead,
        "Map writes keep the whole-write flag of maps they leave out."
    );
    assert_eq!(checked.studio.pages[0].title, "Overview");
    let widgets = &checked.studio.pages[0].widgets;
    assert_eq!(widgets.len(), 5);
    assert!(
        matches!(&widgets[1], StudioWidget::CodeCompare { before, after, .. }
        if before.side == CodeSide::Base && after.lines() == (11..=12))
    );
    assert!(matches!(&widgets[2], StudioWidget::Table(table) if table.role == TableRole::Trace));
    assert!(matches!(&widgets[3], StudioWidget::Scope(scope)
        if scope.changes == ["Writes keep the flag"] && scope.mechanical.is_empty()));
    assert!(matches!(&widgets[4], StudioWidget::Tour(tour)
        if tour.steps[0].weight == StepWeight::Core
            && tour.steps[0].anchor.as_ref().is_some_and(|anchor| anchor.lines() == (12..=12))
            && tour.steps[1].anchor.is_none()));

    // The out-of-diff excerpt, the swapped comparison, the ragged table, the
    // reading order with a step outside the diff and the empty scope are rejected.
    let rejected: Vec<&str> = checked
        .rejected
        .iter()
        .map(|rejected| rejected.widget.as_str())
        .collect();
    assert_eq!(
        rejected,
        [
            "Overview: code excerpt “Outside”",
            "Overview: code comparison “Swapped”",
            "Overview: table “Ragged”",
            "Overview: reading order “Lost”",
            "Overview: scope"
        ]
    );
    assert!(
        checked.rejected[0]
            .reason
            .contains("head lines shown: 10–13")
    );
}

/// Output that does not have the schema's shape is an error, not an empty studio.
#[test]
fn refuses_output_of_another_shape() {
    let targets = studio_prompt(&request()).targets;
    assert!(parse_studio(json!({ "summary": "x" }), &targets).is_err());
    assert!(
        parse_studio(
            json!({ "lead": "x", "pages": [{ "title": "T", "summary": "S", "widgets": [{ "kind": "poem" }] }] }),
            &targets
        )
        .is_err()
    );
}

/// The schema handed to the agents is valid JSON with the two top-level fields required.
#[test]
fn schema_is_json() {
    let schema: serde_json::Value = serde_json::from_str(STUDIO_SCHEMA).expect("valid JSON");
    assert_eq!(schema["required"], json!(["lead", "pages"]));
}

/// A graph node with the given id, group and change.
fn node(id: &str, group: Option<&str>, change: &str) -> serde_json::Value {
    json!({ "id": id, "label": id.to_uppercase(), "detail": "", "role": "actor",
            "group": group, "anchor": null, "change": change })
}

/// An edge between two node ids.
fn edge(from: &str, to: &str, label: &str, change: &str) -> serde_json::Value {
    json!({ "from": from, "to": to, "label": label, "style": "control", "change": change })
}

/// Graphs and sequences keep their parts with ids turned into indexes; ones
/// naming unknown ids, repeating ids, leaving arrows unnamed or joining a
/// part missing from a picture are rejected.
#[test]
fn checks_graphs_and_sequences() {
    let targets = studio_prompt(&request()).targets;
    let graph = |nodes: Vec<serde_json::Value>, edges: Vec<serde_json::Value>| {
        json!({ "kind": "graph", "title": "Owners",
                "groups": [{ "id": "world", "label": "World thread" }],
                "nodes": nodes, "edges": edges })
    };
    let output = json!({
        "lead": "Lead.",
        "pages": [{ "title": "Diagrams", "summary": "S", "widgets": [
            graph(
                vec![node("a", None, "same"), node("b", Some("world"), "added")],
                vec![edge("a", "b", "job", "added")],
            ),
            graph(vec![node("a", None, "same"), node("b", None, "same")], vec![edge("a", "c", "job", "same")]),
            graph(vec![node("a", None, "same"), node("a", None, "same")], vec![]),
            graph(vec![node("a", None, "same"), node("b", None, "same")], vec![edge("a", "b", " ", "same")]),
            graph(
                vec![node("a", None, "same"), node("b", None, "removed")],
                vec![edge("a", "b", "job", "same")],
            ),
            { "kind": "sequence", "title": "Refresh",
              "lanes": [{ "id": "queue", "label": "Queue", "role": "actor" },
                        { "id": "github", "label": "GitHub", "role": "external" }],
              "messages": [
                { "from": "queue", "to": "github", "label": "GET /search", "style": "call",
                  "note": " ", "anchor": anchor("head", 11, 12) },
                { "from": "github", "to": "queue", "label": "403", "style": "reply",
                  "note": "budget used up", "anchor": null }
              ] }
        ] }]
    });

    let checked = parse_studio(output, &targets).expect("matches the schema");

    // The valid graph and the sequence are kept, ids as indexes.
    assert_eq!(
        checked.studio.pages[0].widgets.len(),
        2,
        "{:?}",
        checked.rejected
    );
    let StudioWidget::Graph(kept) = &checked.studio.pages[0].widgets[0] else {
        panic!("a graph comes first");
    };
    assert!(kept.has_changes());
    assert_eq!(kept.nodes[1].group, Some(0));
    assert_eq!((kept.edges[0].from, kept.edges[0].to), (0, 1));
    let StudioWidget::Sequence(sequence) = &checked.studio.pages[0].widgets[1] else {
        panic!("the sequence comes second");
    };
    assert_eq!((sequence.messages[1].from, sequence.messages[1].to), (1, 0));
    assert_eq!(sequence.messages[0].note, None, "a blank note is dropped");

    // The four broken graphs are rejected, each with its reason.
    let reasons: Vec<_> = checked
        .rejected
        .iter()
        .map(|rejected| rejected.reason.as_str())
        .collect();
    assert!(reasons[0].contains("unknown node “c”"), "{reasons:?}");
    assert!(reasons[1].contains("used twice"), "{reasons:?}");
    assert!(reasons[2].contains("has no label"), "{reasons:?}");
    assert!(reasons[3].contains("missing from"), "{reasons:?}");
}

/// Pages keep their order and trimmed titles; pages past the tenth are
/// left out, each listed as rejected.
#[test]
fn keeps_at_most_ten_pages() {
    let targets = studio_prompt(&request()).targets;
    let pages: Vec<_> = (1..=12)
        .map(|number| json!({ "title": format!("Page {number}"), "summary": "S", "widgets": [] }))
        .collect();
    let checked =
        parse_studio(json!({ "lead": "L", "pages": pages }), &targets).expect("matches the schema");
    let titles: Vec<_> = checked
        .studio
        .pages
        .iter()
        .map(|page| page.title.as_str())
        .collect();
    assert_eq!(titles.len(), 10);
    assert_eq!(titles.last(), Some(&"Page 10"));
    assert_eq!(checked.rejected.len(), 2);
    assert!(checked.rejected[0].widget.contains("Page 11"));
}
