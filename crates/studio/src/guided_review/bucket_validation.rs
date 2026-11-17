use std::collections::{HashMap, HashSet};

use crate::SmartReviewItem;

use super::guide_limits::MAX_CONTENT_BYTES;
use super::{BucketCandidate, GuideIssue, GuideIssueCode, GuideIssues};

/// The first selector that introduced one exact member of a proposed item.
struct FileSource {
    path: String,
    item_index: usize,
    selector_index: usize,
}

/// Charges transient selector expansion before cloning paths or growing collections.
struct ExpansionBudget {
    remaining_bytes: usize,
}

impl ExpansionBudget {
    fn new(input_bytes: usize) -> Self {
        Self {
            remaining_bytes: MAX_CONTENT_BYTES.saturating_sub(input_bytes),
        }
    }

    /// Counts the source, owned result, ownership map and their path text conservatively.
    fn reserve_path(&mut self, path: &str) -> Result<(), GuideIssues> {
        let bytes = path.len().saturating_mul(3).saturating_add(
            size_of::<FileSource>() + 3 * size_of::<String>() + 4 * size_of::<usize>(),
        );
        let Some(remaining) = self.remaining_bytes.checked_sub(bytes) else {
            let mut issues = GuideIssues::default();
            issues.push(GuideIssue::new(
                GuideIssueCode::ResourceLimit,
                "Selector expansion exceeds the 16 MiB working-memory budget. Use fewer overlapping items or narrower complete groups.",
            ));
            return Err(issues);
        };
        self.remaining_bytes = remaining;
        Ok(())
    }
}

/// Checks guidance and all expanded membership before accepting any of a bucket.
pub(crate) fn validate_bucket(
    bucket_id: &str,
    candidate: &BucketCandidate,
    changed_paths: &[String],
) -> Result<Vec<SmartReviewItem>, GuideIssues> {
    let mut issues = GuideIssues::default();
    check_guidance(
        bucket_id,
        &candidate.label,
        &candidate.description,
        None,
        &mut issues,
    );
    if candidate.items.is_empty() {
        let mut issue = bucket_issue(
            bucket_id,
            GuideIssueCode::EmptyItems,
            "A review bucket needs at least one reading item.",
        );
        issue.locations.push("/items".into());
        issues.push(issue);
    }

    // Each item deduplicates its own selectors; a second item cannot claim its files.
    let mut owners = HashMap::<String, FileSource>::new();
    let mut budget = ExpansionBudget::new(candidate.storage_bytes());
    let mut items = Vec::new();
    for (item_index, item) in candidate.items.iter().enumerate() {
        check_guidance(
            bucket_id,
            &item.label,
            &item.description,
            Some(item_index),
            &mut issues,
        );
        let sources = expand_selectors(
            bucket_id,
            item_index,
            &item.paths,
            changed_paths,
            &mut issues,
            &mut budget,
        )?;
        let files = sources.iter().map(|source| source.path.clone()).collect();
        for source in sources {
            if let Some(previous) = owners.get(&source.path) {
                let mut issue = bucket_issue(
                    bucket_id,
                    GuideIssueCode::DuplicateFileInBucket,
                    "This changed file belongs to two items after selector expansion. Keep it in one item of this bucket.",
                );
                issue.path = Some(source.path.clone());
                issue.item_indexes = vec![previous.item_index, item_index];
                issue.locations = vec![selector_pointer(previous), selector_pointer(&source)];
                issue.selectors = vec![
                    candidate.items[previous.item_index].paths[previous.selector_index].clone(),
                    item.paths[source.selector_index].clone(),
                ];
                issues.push(issue);
            } else {
                owners.insert(source.path.clone(), source);
            }
        }
        items.push(SmartReviewItem {
            label: item.label.trim().into(),
            description: item.description.trim().into(),
            files,
        });
    }
    if issues.is_empty() {
        Ok(items)
    } else {
        Err(issues)
    }
}

/// Legacy bucket selectors expand with the same rules, then become individual stops.
pub(crate) fn validate_legacy_paths(
    bucket_id: &str,
    label: &str,
    description: &str,
    paths: &[String],
    changed_paths: &[String],
) -> Result<Vec<SmartReviewItem>, GuideIssues> {
    let mut issues = GuideIssues::default();
    check_guidance(bucket_id, label, description, None, &mut issues);
    let mut budget = ExpansionBudget::new(0);
    let sources = expand_selectors(bucket_id, 0, paths, changed_paths, &mut issues, &mut budget)?;
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(sources
        .into_iter()
        .map(|source| SmartReviewItem {
            label: source.path.clone(),
            description: String::new(),
            files: vec![source.path],
        })
        .collect())
}

/// Reports each unknown selector while preserving selector order and inventory order.
fn expand_selectors(
    bucket_id: &str,
    item_index: usize,
    selectors: &[String],
    changed_paths: &[String],
    issues: &mut GuideIssues,
    budget: &mut ExpansionBudget,
) -> Result<Vec<FileSource>, GuideIssues> {
    let mut sources = Vec::new();
    let mut seen = HashSet::new();
    for (selector_index, selector) in selectors.iter().enumerate() {
        let mut matched = false;
        for path in changed_paths.iter().filter(|path| {
            *path == selector || (selector.ends_with('/') && path.starts_with(selector))
        }) {
            matched = true;
            if seen.insert(path) {
                budget.reserve_path(path)?;
                sources.push(FileSource {
                    path: path.clone(),
                    item_index,
                    selector_index,
                });
            }
        }
        if !matched {
            let mut issue = bucket_issue(
                bucket_id,
                GuideIssueCode::UnknownPath,
                "This selector matches no changed path in the captured inventory.",
            );
            issue.path = Some(selector.clone());
            issue.item_indexes.push(item_index);
            issue
                .locations
                .push(format!("/items/{item_index}/paths/{selector_index}"));
            issue.selectors.push(selector.clone());
            issues.push(issue);
        }
    }
    if selectors.is_empty() {
        let mut issue = bucket_issue(
            bucket_id,
            GuideIssueCode::EmptyPaths,
            "A reading item needs at least one changed-file selector.",
        );
        issue.item_indexes.push(item_index);
        issue.locations.push(format!("/items/{item_index}/paths"));
        issues.push(issue);
    }
    Ok(sources)
}

/// Empty text is rejected without stopping independent path diagnostics.
fn check_guidance(
    bucket_id: &str,
    label: &str,
    description: &str,
    item_index: Option<usize>,
    issues: &mut GuideIssues,
) {
    let prefix = match item_index {
        Some(index) => format!("/items/{index}"),
        None => String::new(),
    };
    let mut issue = bucket_issue(
        bucket_id,
        GuideIssueCode::EmptyGuidance,
        "Every bucket and reading item needs a nonempty label and reading guidance.",
    );
    if label.trim().is_empty() {
        issue.locations.push(format!("{prefix}/label"));
    }
    if description.trim().is_empty() {
        issue.locations.push(format!("{prefix}/description"));
    }
    if !issue.locations.is_empty() {
        issue.item_indexes.extend(item_index);
        issues.push(issue);
    }
}

fn bucket_issue(bucket_id: &str, code: GuideIssueCode, message: &str) -> GuideIssue {
    let mut issue = GuideIssue::new(code, message);
    issue.bucket_id = Some(bucket_id.into());
    issue
}

fn selector_pointer(source: &FileSource) -> String {
    format!(
        "/items/{}/paths/{}",
        source.item_index, source.selector_index
    )
}
