use qd_diff::{DiffOptions, FileDiff};
use serde_json::{Value, json};

use crate::AnchorTargets;
use crate::brief_draft::BriefDraft;

/// A renamed file with distinct old/new line ranges.
pub(super) fn targets() -> AnchorTargets {
    let mut targets = AnchorTargets::default();
    targets.add_file(
        "src/lib.rs",
        Some("src/old.rs"),
        &FileDiff::build(
            "@@ -10,3 +10,4 @@\n a\n-b\n+c\n+d\n e",
            DiffOptions::default(),
        ),
    );
    targets
}

pub(super) fn draft() -> BriefDraft {
    BriefDraft::new("run".into(), "head:base".into(), targets())
}

pub(super) fn page(widgets: Vec<Value>) -> Value {
    json!({"title":"Behavior","summary":"What the change does.","widgets":Value::Array(widgets)})
}

pub(super) fn section() -> Value {
    json!({"kind":"section","heading":"Changed behavior","body":"The caller now retries."})
}

pub(super) fn excerpt(end: u32) -> Value {
    json!({"kind":"code_excerpt","caption":"Follow the branch.","anchor":{"path":"src/lib.rs","side":"head","start_line":10,"end_line":end}})
}

pub(super) fn order(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| (*id).into()).collect()
}
