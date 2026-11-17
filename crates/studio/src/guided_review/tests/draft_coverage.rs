use crate::guided_review::{DraftStatus, GuideIssue, GuideIssueCode, GuidedReviewDraft};

use super::fixtures::{candidate, draft, order};

/// An incomplete first bucket is accepted; finishing names binary, moved and generated omissions.
#[test]
fn completion_requires_every_captured_path_and_explicit_bucket_order() {
    let mut draft = draft(&[
        "code.rs",
        "asset.bin",
        "generated/file",
        "moved.rs",
        "code.rs",
    ]);
    let first = draft.replace_bucket("code", 0, "first", candidate(&[&["code.rs"]]));
    assert_eq!(first.status, DraftStatus::Accepted);
    assert_eq!(first.coverage.total_paths, 4);
    assert_eq!(first.coverage.missing_paths, 3);
    let invalid = draft.finish_guide(1, &order(&["code"]), "too-early");
    assert_eq!(invalid.issue_count, 3);
    assert!(
        invalid
            .issues
            .iter()
            .all(|issue| issue.code == GuideIssueCode::MissingPaths)
    );
    assert_eq!(
        draft.missing_paths(0, 2).paths,
        ["asset.bin", "generated/file"]
    );
    assert_eq!(draft.missing_paths(0, 2).next_offset, Some(2));
    assert_eq!(draft.missing_paths(2, 2).paths, ["moved.rs"]);

    draft.replace_bucket(
        "assets",
        1,
        "rest",
        candidate(&[&["asset.bin", "generated/", "moved.rs"]]),
    );
    for invalid_order in [
        order(&["code"]),
        order(&["code", "code", "assets"]),
        order(&["code", "assets", "unknown"]),
    ] {
        assert!(
            draft
                .validate_guide(2, &invalid_order)
                .issues
                .iter()
                .any(|issue| issue.code == GuideIssueCode::InvalidOrder)
        );
    }
    let finished = draft.finish_guide(2, &order(&["assets", "code"]), "done");
    let buckets = draft
        .finished_buckets(finished.receipt.unwrap().as_str())
        .unwrap();
    assert_eq!(
        buckets[0].files().collect::<Vec<_>>(),
        ["asset.bin", "generated/file", "moved.rs"]
    );
    assert_eq!(buckets[1].order, 2);
}

/// Thousands of inventory entries page completely and one bucket repair leaves the others unchanged.
#[test]
fn repairs_one_bucket_in_large_inventory() {
    let mut paths = vec![
        "contract.rs".to_owned(),
        "tests.rs".to_owned(),
        "asset.bin".to_owned(),
    ];
    paths.extend((0..3800).map(|index| format!("fixtures/{index}.json")));
    let mut draft = GuidedReviewDraft::new("large".into(), "snapshot".into(), paths.clone());
    let mut offset = 0;
    let mut all_paths = Vec::new();
    loop {
        let page = draft.inventory(offset, usize::MAX);
        assert!(page.paths.len() <= 256);
        all_paths.extend(page.paths);
        let Some(next_offset) = page.next_offset else {
            break;
        };
        offset = next_offset;
    }
    assert_eq!(all_paths, paths);

    let fixtures = candidate(&[&["fixtures/", "asset.bin"]]);
    draft.replace_bucket("assets", 0, "assets", fixtures.clone());
    let incomplete = draft.validate_guide(1, &order(&["assets"]));
    assert_eq!(incomplete.coverage.missing_paths, 2);
    draft.replace_bucket(
        "contract",
        1,
        "wrong",
        candidate(&[&["contract.rs", "tests.rs"], &["tests.rs"]]),
    );
    assert_eq!(draft.draft().revision, 1);
    draft.replace_bucket(
        "contract",
        1,
        "fixed",
        candidate(&[&["tests.rs"], &["contract.rs"]]),
    );
    assert_eq!(draft.bucket("assets"), Some(&fixtures));
    assert_eq!(
        draft
            .validate_guide(2, &order(&["contract", "assets"]))
            .status,
        DraftStatus::Valid
    );
}

/// Diagnostics retain exact missing totals while bounding the concrete issue sample.
#[test]
fn bounds_diagnostics_without_losing_missing_counts() {
    let paths: Vec<_> = (0..4000).map(|index| format!("file-{index}")).collect();
    let draft = GuidedReviewDraft::new("large".into(), "snapshot".into(), paths);
    let result = draft.validate_guide(0, &[]);
    assert_eq!(result.coverage.missing_paths, 4000);
    assert_eq!(result.issue_count, 4001);
    assert!(result.issues.len() <= 64);
    assert!(
        result
            .issues
            .iter()
            .map(GuideIssue::encoded_bytes)
            .sum::<usize>()
            <= 64 * 1024
    );
}

/// Bucket count is not an arbitrary model-output limit and cross-bucket overlap remains valid.
#[test]
fn supports_many_valid_buckets() {
    let mut draft = draft(&["a"]);
    let mut ids = Vec::new();
    for index in 0..80_u64 {
        let id = format!("bucket-{index}");
        let result = draft.replace_bucket(&id, index, &id, candidate(&[&["a"]]));
        assert_eq!(result.status, DraftStatus::Accepted);
        ids.push(id);
    }
    assert_eq!(draft.validate_guide(80, &ids).status, DraftStatus::Valid);
}
