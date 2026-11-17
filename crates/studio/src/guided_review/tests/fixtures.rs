use crate::guided_review::{BucketCandidate, GuidedReviewDraft, ItemCandidate};

/// Creates a fixed run identity with the scenario's exact changed paths.
pub(super) fn draft(paths: &[&str]) -> GuidedReviewDraft {
    GuidedReviewDraft::new(
        "run-one".into(),
        "head:base".into(),
        paths.iter().map(|path| (*path).into()).collect(),
    )
}

/// Makes valid guidance so each scenario can focus on its proposed membership.
pub(super) fn candidate(groups: &[&[&str]]) -> BucketCandidate {
    BucketCandidate {
        label: "Runtime".into(),
        description: "Read the contract and behavior.".into(),
        suggest_viewed: false,
        items: groups
            .iter()
            .enumerate()
            .map(|(index, paths)| ItemCandidate {
                label: format!("Step {index}"),
                description: "Follow this change.".into(),
                paths: paths.iter().map(|path| (*path).into()).collect(),
            })
            .collect(),
    }
}

/// Owns a scenario's explicit reading order independently of bucket IDs.
pub(super) fn order(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| (*id).into()).collect()
}
