//! The GraphQL query of a pull request's merge options, and the REST
//! answers of an asynchronous merge.

use serde::Deserialize;

use crate::{
    MergeMethod, MergeOptions, MergeOutcome, MergePermissions, MergeQueueState, MergeReadiness,
    PullRequestState,
};

/// A pull request's state, readiness, the repository's merge settings and
/// the viewer's permissions. One GraphQL request.
pub(crate) const MERGE_OPTIONS_QUERY: &str = r"
query MergeOptions($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    mergeCommitAllowed squashMergeAllowed rebaseMergeAllowed deleteBranchOnMerge
    pullRequest(number: $number) {
      state mergeable mergeStateStatus isInMergeQueue isMergeQueueEnabled
      headRefName headRefOid baseRefName isCrossRepository
      viewerCanMergeAsAdmin viewerCanClose viewerCanDeleteHeadRef
    }
  }
}
";

/// The data of [`MERGE_OPTIONS_QUERY`].
#[derive(Deserialize)]
pub(crate) struct MergeOptionsData {
    repository: Option<RepositoryNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryNode {
    #[serde(flatten)]
    methods: AllowedMethodsNode,
    delete_branch_on_merge: bool,
    pull_request: Option<PullRequestNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AllowedMethodsNode {
    merge_commit_allowed: bool,
    squash_merge_allowed: bool,
    rebase_merge_allowed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRequestNode {
    state: String,
    mergeable: String,
    merge_state_status: String,
    #[serde(flatten)]
    queue: QueueNode,
    head_ref_name: String,
    head_ref_oid: String,
    base_ref_name: String,
    is_cross_repository: bool,
    #[serde(flatten)]
    permissions: PermissionsNode,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueueNode {
    is_in_merge_queue: bool,
    is_merge_queue_enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PermissionsNode {
    viewer_can_merge_as_admin: bool,
    viewer_can_close: bool,
    viewer_can_delete_head_ref: bool,
}

impl MergeOptionsData {
    /// The merge options, or why there are none (no such pull request).
    pub fn into_options(self) -> Result<MergeOptions, String> {
        let repository = self.repository.ok_or("GitHub knows no such repository")?;
        let pull_request = repository
            .pull_request
            .ok_or("GitHub knows no such pull request")?;
        let methods = repository.methods;
        let allowed_methods = [
            (methods.merge_commit_allowed, MergeMethod::Merge),
            (methods.squash_merge_allowed, MergeMethod::Squash),
            (methods.rebase_merge_allowed, MergeMethod::Rebase),
        ]
        .into_iter()
        .filter_map(|(is_allowed, method)| is_allowed.then_some(method))
        .collect();
        Ok(MergeOptions {
            state: match pull_request.state.as_str() {
                "MERGED" => PullRequestState::Merged,
                "CLOSED" => PullRequestState::Closed,
                _ => PullRequestState::Open,
            },
            readiness: MergeReadiness::from_github(
                &pull_request.merge_state_status,
                &pull_request.mergeable,
            ),
            allowed_methods,
            permissions: MergePermissions {
                can_merge_as_admin: pull_request.permissions.viewer_can_merge_as_admin,
                can_close: pull_request.permissions.viewer_can_close,
                can_delete_head_ref: pull_request.permissions.viewer_can_delete_head_ref,
            },
            deletes_branch_on_merge: repository.delete_branch_on_merge,
            merge_queue: match (
                pull_request.queue.is_merge_queue_enabled,
                pull_request.queue.is_in_merge_queue,
            ) {
                (_, true) => MergeQueueState::Queued,
                (true, false) => MergeQueueState::Available,
                (false, false) => MergeQueueState::NoQueue,
            },
            head_ref: pull_request.head_ref_name,
            head_sha: pull_request.head_ref_oid,
            base_ref: pull_request.base_ref_name,
            is_cross_repository: pull_request.is_cross_repository,
        })
    }
}

/// The answer of starting or polling an asynchronous merge.
#[derive(Deserialize)]
pub(crate) struct MergeAsyncResponse {
    status: String,
    #[serde(default)]
    details: Option<MergeAsyncDetails>,
}

#[derive(Deserialize)]
struct MergeAsyncDetails {
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

impl MergeAsyncResponse {
    /// The outcome; a pending merge without a request id is a failure to report.
    pub fn into_outcome(self) -> MergeOutcome {
        let details = self.details;
        let message = details.as_ref().and_then(|details| details.message.clone());
        match self.status.as_str() {
            "merged" => MergeOutcome::Merged,
            "enqueued" => MergeOutcome::Enqueued,
            "pending" => match details.and_then(|details| details.uuid) {
                Some(uuid) => MergeOutcome::Pending { uuid },
                None => MergeOutcome::Failed("GitHub sent no merge request id".to_owned()),
            },
            other => MergeOutcome::Failed(message.unwrap_or_else(|| format!("merge {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphql_response::GraphQlResponse;

    /// The options keep only allowed methods and read readiness and permissions.
    #[test]
    fn reads_merge_options() {
        let json = r#"{"data": {"repository": {
            "mergeCommitAllowed": false, "squashMergeAllowed": true, "rebaseMergeAllowed": false,
            "deleteBranchOnMerge": true,
            "pullRequest": {
                "state": "OPEN", "isDraft": false, "mergeable": "MERGEABLE", "mergeStateStatus": "BLOCKED",
                "isInMergeQueue": false, "isMergeQueueEnabled": false,
                "headRefName": "feature", "headRefOid": "abc", "baseRefName": "main", "isCrossRepository": false,
                "viewerCanMergeAsAdmin": true, "viewerCanClose": true, "viewerCanDeleteHeadRef": false
            }
        }}}"#;
        let response: GraphQlResponse<MergeOptionsData> = serde_json::from_str(json).unwrap();
        let options = response.into_data().unwrap().into_options().unwrap();
        assert_eq!(options.allowed_methods, [MergeMethod::Squash]);
        assert_eq!(options.readiness, MergeReadiness::Blocked);
        assert!(options.permissions.can_merge_as_admin && !options.permissions.can_delete_head_ref);
    }

    /// Conflicts win over the merge state.
    #[test]
    fn conflicts_win() {
        assert_eq!(
            MergeReadiness::from_github("BLOCKED", "CONFLICTING"),
            MergeReadiness::Conflicting
        );
    }

    /// A pending answer keeps its request id; a failure keeps GitHub's message.
    #[test]
    fn reads_async_merge_answers() {
        let pending: MergeAsyncResponse = serde_json::from_str(
            r#"{"status": "pending", "details": {"uuid": "u-1", "message": "Queued"}}"#,
        )
        .unwrap();
        assert_eq!(
            pending.into_outcome(),
            MergeOutcome::Pending { uuid: "u-1".into() }
        );
        let failed: MergeAsyncResponse = serde_json::from_str(
            r#"{"status": "failed", "details": {"message": "Required checks failed"}}"#,
        )
        .unwrap();
        assert_eq!(
            failed.into_outcome(),
            MergeOutcome::Failed("Required checks failed".into())
        );
    }
}
