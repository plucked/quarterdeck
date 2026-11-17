use std::collections::HashSet;

use crate::SmartReviewItem;
use crate::guided_review::{
    BucketCandidate, GuideIssueCode, GuideIssues, ItemCandidate, validate_bucket,
    validate_legacy_paths,
};
use serde::Deserialize;

/// A saved review plan. Membership is expanded against the original diff,
/// so an old folder selector never silently includes newly changed files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartFilter {
    pub head_sha: String,
    pub base_sha: Option<String>,
    pub generated_at: u64,
    pub buckets: Vec<SmartBucket>,
}

/// One step in the recommended reading order, with exact changed-file paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartBucket {
    pub order: u32,
    pub label: String,
    pub description: String,
    pub suggest_viewed: bool,
    pub files: Vec<String>,
    pub items: Vec<SmartReviewItem>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Answer {
    buckets: Vec<BucketAnswer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BucketAnswer {
    order: u32,
    label: String,
    description: String,
    suggest_viewed: bool,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    items: Vec<ItemAnswer>,
}

/// Ordered reading items in the legacy one-shot answer, before domain validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemAnswer {
    label: String,
    description: String,
    paths: Vec<String>,
}

impl SmartFilter {
    pub fn is_stale(&self, head_sha: &str, base_sha: Option<&str>) -> bool {
        self.head_sha != head_sha || self.base_sha.as_deref() != base_sha
    }
}

/// Checks the agent's answer before it can filter or mark files. A selector
/// is an exact file or a folder ending in `/`; every changed file must be
/// covered. Overlapping buckets are allowed, but empty/unknown ones are not.
pub fn parse_smart_buckets(
    output: serde_json::Value,
    changed_paths: &[&str],
) -> Result<Vec<SmartBucket>, String> {
    let answer: Answer = serde_json::from_value(output).map_err(|error| error.to_string())?;
    if answer.buckets.is_empty() {
        return Err("The agent returned no review buckets.".into());
    }
    // Capture unique exact paths once; both adapters share selector and item validation.
    let mut inventory_seen = HashSet::new();
    let inventory: Vec<String> = changed_paths
        .iter()
        .copied()
        .filter(|path| inventory_seen.insert(*path))
        .map(str::to_owned)
        .collect();
    let mut orders = HashSet::new();
    let mut covered = HashSet::new();
    let mut buckets = Vec::new();
    for bucket in answer.buckets {
        if bucket.order == 0 || !orders.insert(bucket.order) {
            return Err("Review buckets need unique, positive order numbers.".into());
        }
        if !bucket.paths.is_empty() && !bucket.items.is_empty() {
            return Err("A bucket must use ordered items or legacy paths, not both.".into());
        }
        let bucket_id = bucket.order.to_string();
        let items = validated_bucket_items(&bucket_id, &bucket, &inventory)
            .map_err(|issues| legacy_validation_message(&issues))?;
        let files: Vec<String> = items.iter().flat_map(|item| item.files.clone()).collect();
        covered.extend(files.iter().cloned());
        buckets.push(SmartBucket {
            order: bucket.order,
            label: bucket.label.trim().to_owned(),
            description: bucket.description.trim().to_owned(),
            suggest_viewed: bucket.suggest_viewed,
            files,
            items,
        });
    }
    // Set membership, rather than equal cardinality, is the completion contract.
    if inventory.iter().any(|path| !covered.contains(path)) {
        return Err(
            "The agent's review buckets left out changed files. Generate again to retry.".into(),
        );
    }
    buckets.sort_by_key(|bucket| bucket.order);
    Ok(buckets)
}

/// Decodes the legacy alternative before delegating all guidance and membership rules.
fn validated_bucket_items(
    bucket_id: &str,
    bucket: &BucketAnswer,
    inventory: &[String],
) -> Result<Vec<SmartReviewItem>, GuideIssues> {
    if bucket.items.is_empty() {
        return validate_legacy_paths(
            bucket_id,
            &bucket.label,
            &bucket.description,
            &bucket.paths,
            inventory,
        );
    }
    validate_bucket(
        bucket_id,
        &BucketCandidate {
            label: bucket.label.clone(),
            description: bucket.description.clone(),
            suggest_viewed: bucket.suggest_viewed,
            items: bucket
                .items
                .iter()
                .map(|item| ItemCandidate {
                    label: item.label.clone(),
                    description: item.description.clone(),
                    paths: item.paths.clone(),
                })
                .collect(),
        },
        inventory,
    )
}

/// Keeps the one-shot adapter's familiar errors while checks remain shared with draft repair.
fn legacy_validation_message(issues: &GuideIssues) -> String {
    let Some(issue) = issues.issues.first() else {
        return "The review bucket has invalid guidance or file membership.".into();
    };
    match issue.code {
        GuideIssueCode::EmptyGuidance if issue.item_indexes.is_empty() => {
            "A review bucket has no label or description.".into()
        }
        GuideIssueCode::EmptyGuidance => {
            "Every walkthrough item needs a label and reading guidance.".into()
        }
        GuideIssueCode::DuplicateFileInBucket => {
            "A file appears in more than one item of a review bucket.".into()
        }
        GuideIssueCode::UnknownPath => format!(
            "A walkthrough references an unknown path: {}",
            issue.path.as_deref().unwrap_or("")
        ),
        GuideIssueCode::EmptyPaths | GuideIssueCode::EmptyItems => {
            "A walkthrough item contains no changed files.".into()
        }
        _ => issue.message.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn bucket(order: u32, paths: &[&str]) -> serde_json::Value {
        json!({"order": order, "label": "Core", "description": "Read the new behavior.",
            "suggest_viewed": false, "paths": paths})
    }

    #[test]
    fn expands_folders_once_orders_and_deduplicates_members() {
        let result = parse_smart_buckets(
            json!({"buckets": [
                bucket(2, &["src/", "src/a.rs"]), bucket(1, &["src-old/b.rs"])
            ]}),
            &["src/a.rs", "src/nested/c.rs", "src-old/b.rs"],
        )
        .unwrap();
        assert_eq!(result[0].order, 1);
        assert_eq!(result[1].files, ["src/a.rs", "src/nested/c.rs"]);
    }

    #[test]
    fn refuses_unknown_missing_empty_and_duplicate_order_buckets() {
        for buckets in [
            vec![],
            vec![bucket(1, &["missing"])],
            vec![bucket(1, &[])],
            vec![bucket(1, &["a"])],
            vec![bucket(1, &["a"]), bucket(1, &["b"])],
        ] {
            assert!(parse_smart_buckets(json!({"buckets": buckets}), &["a", "b"]).is_err());
        }
    }

    #[test]
    fn overlapping_buckets_and_arbitrary_bucket_counts_are_supported() {
        let buckets: Vec<_> = (1..=80).map(|order| bucket(order, &["a"])).collect();
        assert_eq!(
            parse_smart_buckets(json!({"buckets": buckets}), &["a"])
                .unwrap()
                .len(),
            80
        );
    }

    #[test]
    fn either_commit_changing_makes_the_plan_stale() {
        let filter = SmartFilter {
            head_sha: "head".into(),
            base_sha: Some("base".into()),
            generated_at: 0,
            buckets: vec![],
        };
        assert!(!filter.is_stale("head", Some("base")));
        assert!(filter.is_stale("new", Some("base")));
        assert!(filter.is_stale("head", Some("new")));
    }
}

#[cfg(test)]
mod walkthrough_tests {
    use super::*;
    use serde_json::json;

    /// Hundreds of explicit fixture members remain one ordered stop after the prerequisite file.
    #[test]
    fn preserves_order_and_explicit_asset_groups() {
        let mut paths = vec!["format.md".to_owned(), "tests.rs".to_owned()];
        paths.extend((0..300).map(|index| format!("fixtures/{index}.bin")));
        let borrowed: Vec<_> = paths.iter().map(String::as_str).collect();
        let result = parse_smart_buckets(json!({"buckets":[{
            "order":1,"label":"Contract","description":"Check the format and its tests.","suggest_viewed":false,
            "items":[
                {"label":"Tests","description":"Verify decoding before acknowledging fixtures.","paths":["tests.rs"]},
                {"label":"Format","description":"Read the format.","paths":["format.md"]},
                {"label":"Binary fixtures","description":"Shared vectors, acknowledged together.","paths":["fixtures/"]}
            ]
        }]}), &borrowed).unwrap();
        assert_eq!(result[0].items.len(), 3);
        assert_eq!(result[0].files[0], "tests.rs");
        assert_eq!(result[0].items[2].files.len(), 300);
        assert!(!result[0].suggest_viewed);
    }

    /// Invalid guidance, membership or item wire fields cannot become actionable groups.
    #[test]
    fn rejects_invalid_walkthroughs() {
        for items in [
            json!([{"label":"A","description":"Read","paths":["a"]},{"label":"Again","description":"Read","paths":["a"]}]),
            json!([{"label":"A","description":"","paths":["a"]}]),
            json!([{"label":"A","description":"Read","paths":["unknown"]}]),
            json!([{"label":"A","paths":["a"]}]),
            json!([{"label":"A","description":"Read","paths":["a"],"unexpected":true}]),
        ] {
            assert!(parse_smart_buckets(json!({"buckets":[{"order":1,"label":"Core","description":"Read","suggest_viewed":false,"items":items}]}), &["a"]).is_err());
        }
    }
}
