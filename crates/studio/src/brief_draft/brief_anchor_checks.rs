use serde_json::Value;

use crate::wire_anchor::WireAnchor;
use crate::{AnchorTargets, CodeSide};

use super::{BriefIssue, BriefIssues};

/// Walks all anchor locations, reusing the same anchor methods as widget construction.
pub(crate) fn check_page_anchors(
    candidate: &Value,
    targets: &AnchorTargets,
    issues: &mut BriefIssues,
) {
    let Some(widgets) = candidate["widgets"].as_array() else {
        return;
    };
    for (index, widget) in widgets.iter().enumerate() {
        let prefix = format!("/widgets/{index}");
        match widget["kind"].as_str() {
            Some("code_excerpt") => check_anchor(
                &widget["anchor"],
                &format!("{prefix}/anchor"),
                targets,
                AnchorUse::Excerpt,
                issues,
            ),
            Some("code_compare") => {
                check_anchor(
                    &widget["before"],
                    &format!("{prefix}/before"),
                    targets,
                    AnchorUse::Excerpt,
                    issues,
                );
                check_anchor(
                    &widget["after"],
                    &format!("{prefix}/after"),
                    targets,
                    AnchorUse::Excerpt,
                    issues,
                );
            }
            Some("tour") => {
                check_collection(widget, "steps", &prefix, targets, AnchorUse::Start, issues);
            }
            Some("graph") => check_collection(
                widget,
                "nodes",
                &prefix,
                targets,
                AnchorUse::Excerpt,
                issues,
            ),
            Some("sequence") => check_collection(
                widget,
                "messages",
                &prefix,
                targets,
                AnchorUse::Excerpt,
                issues,
            ),
            _ => {}
        }
    }
}

/// Tour anchors intentionally cite only the start; other anchors retain their full range.
#[derive(Clone, Copy)]
enum AnchorUse {
    Start,
    Excerpt,
}

fn check_collection(
    widget: &Value,
    field: &str,
    prefix: &str,
    targets: &AnchorTargets,
    usage: AnchorUse,
    issues: &mut BriefIssues,
) {
    if let Some(items) = widget[field].as_array() {
        for (index, item) in items.iter().enumerate() {
            check_anchor(
                &item["anchor"],
                &format!("{prefix}/{field}/{index}/anchor"),
                targets,
                usage,
                issues,
            );
        }
    }
}

/// Bounded structured ranges supplement the ordinary parser's readable anchor error.
fn check_anchor(
    value: &Value,
    location: &str,
    targets: &AnchorTargets,
    usage: AnchorUse,
    issues: &mut BriefIssues,
) {
    if value.is_null() {
        return;
    }
    let Ok(anchor) = serde_json::from_value::<WireAnchor>(value.clone()) else {
        return;
    };
    let result = match usage {
        AnchorUse::Start => anchor.checked_start(targets),
        AnchorUse::Excerpt => anchor.checked(targets),
    };
    if let Err(reason) = result {
        let path = value["path"].as_str().unwrap_or_default();
        let side = match value["side"].as_str() {
            Some("base") => CodeSide::Base,
            _ => CodeSide::Head,
        };
        let ranges = targets.ranges(path, side).unwrap_or_default();
        issues.push(
            BriefIssue::new("invalid_anchor", location, &reason).with_ranges(path, side, ranges),
        );
    }
}
