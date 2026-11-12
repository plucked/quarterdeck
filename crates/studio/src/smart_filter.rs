use std::collections::HashSet;

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
    let mut orders = HashSet::new();
    let mut covered = HashSet::new();
    let mut buckets = Vec::new();
    for bucket in answer.buckets {
        if bucket.order == 0 || !orders.insert(bucket.order) {
            return Err("Review buckets need unique, positive order numbers.".into());
        }
        if bucket.label.trim().is_empty() || bucket.description.trim().is_empty() {
            return Err("A review bucket has no label or description.".into());
        }
        let mut members = HashSet::new();
        for selector in &bucket.paths {
            let matches: Vec<_> = changed_paths
                .iter()
                .copied()
                .filter(|path| {
                    *path == selector || (selector.ends_with('/') && path.starts_with(selector))
                })
                .collect();
            if matches.is_empty() {
                return Err(format!(
                    "A review bucket references an unknown path: {selector}"
                ));
            }
            members.extend(matches);
        }
        if members.is_empty() {
            return Err(format!(
                "The review bucket {} contains no files.",
                bucket.label
            ));
        }
        covered.extend(members.iter().copied());
        buckets.push(SmartBucket {
            order: bucket.order,
            label: bucket.label.trim().to_owned(),
            description: bucket.description.trim().to_owned(),
            suggest_viewed: bucket.suggest_viewed,
            files: changed_paths
                .iter()
                .filter(|path| members.contains(**path))
                .map(|path| (*path).to_owned())
                .collect(),
        });
    }
    if covered.len() != changed_paths.len() {
        return Err(
            "The agent's review buckets left out changed files. Generate again to retry.".into(),
        );
    }
    buckets.sort_by_key(|bucket| bucket.order);
    Ok(buckets)
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
