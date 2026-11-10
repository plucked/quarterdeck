use qd_github::{CheckOutcome, ConflictLocation, MergeConflict};

use super::check_tally::CheckTally;
use super::shown_checks::{CheckContent, ShownChecks};
use crate::display_text::{short_sha, time_ago, unix_now};

/// `This pull request conflicts with develop.`, or the layer below that does.
pub(super) fn conflict_sentence(conflict: &MergeConflict) -> String {
    match conflict.location {
        ConflictLocation::Shown => {
            format!("This pull request conflicts with {}.", conflict.base_ref)
        }
        ConflictLocation::LayerBelow => format!(
            "#{}, lower in this stack, conflicts with {}.",
            conflict.number, conflict.base_ref
        ),
    }
}

/// What a merge conflict means for the checks: none run without a merge
/// commit, so none start until it is resolved. With checks shown, it says
/// which pull request conflicts too (the summary says so only without).
pub(super) fn conflict_note(shown: &ShownChecks) -> Option<String> {
    let conflict = shown.conflict.as_ref()?;
    let has_checks = match shown.content.tree() {
        Some(tree) => tree.tally().total() > tree.expected().len(),
        None => shown.rollup.is_some(),
    };
    let fix = match conflict.location {
        ConflictLocation::Shown => "Resolve it".to_owned(),
        ConflictLocation::LayerBelow => format!("Resolve it in #{}", conflict.number),
    };
    let note = if has_checks {
        format!(
            "{} GitHub starts no pull request workflows while it cannot build the \
             merge commit, so new pushes run no checks. {fix} first.",
            conflict_sentence(conflict)
        )
    } else {
        format!(
            "GitHub starts no pull request workflows while it cannot build the merge \
             commit. {fix} and the checks run."
        )
    };
    Some(note)
}

/// What the token's access leaves out, when it leaves something out.
pub(super) fn access_note(shown: &ShownChecks) -> Option<&'static str> {
    match shown.content {
        CheckContent::NotPermitted => Some(
            "This GitHub token may read only the overall state. To list the checks, \
             give it read access to Actions and Commit statuses.",
        ),
        CheckContent::FromActions(_) => Some(
            "Read from GitHub Actions: this token may not read check runs, so \
             other apps' checks are missing and required checks are matched by name.",
        ),
        CheckContent::Listed(_) => None,
    }
}

/// `Required: 3 of 3 passed · abc1234 · read 2m ago`, the parts known.
pub(super) fn footnote(shown: &ShownChecks, required: Option<&CheckTally>) -> String {
    let read = format!("read {}", time_ago(shown.read_at, unix_now()));
    required
        .map(|required| {
            format!(
                "Required: {} of {} passed",
                required.count(CheckOutcome::Passed),
                required.total()
            )
        })
        .into_iter()
        .chain([short_sha(&shown.head_sha).to_owned(), read])
        .collect::<Vec<_>>()
        .join(" · ")
}
