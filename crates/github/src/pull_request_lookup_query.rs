use std::collections::HashMap;

use serde::Deserialize;

use crate::aliased_pull_requests_query::{AliasedPullRequestsData, AliasedPullRequestsQuery};
use crate::{PullRequestBranches, ViewerReview};

/// The lookup of pull requests' branches and the viewer's latest review.
const LOOKUP_QUERY: AliasedPullRequestsQuery = AliasedPullRequestsQuery {
    operation: "PullRequestLookup",
    alias_prefix: "pr",
    selection: "number headRefName baseRefName viewerLatestReview { state }",
};

/// What a lookup found of one pull request.
pub(crate) struct LookedUpPullRequest {
    pub branches: PullRequestBranches,
    pub viewer_review: Option<ViewerReview>,
}

/// The GraphQL text looking up the branches and the viewer's latest review
/// of pull requests `numbers` of the repository in the `owner` and `name`
/// variables, one aliased `pullRequest(number:)` field each.
pub(crate) fn pull_request_lookup_query(numbers: &[u64]) -> String {
    LOOKUP_QUERY.text(numbers)
}

/// The data of a [`pull_request_lookup_query`].
#[derive(Deserialize)]
#[serde(transparent)]
pub(crate) struct PullRequestLookupData {
    found: AliasedPullRequestsData<LookupNode>,
}

/// One looked-up pull request.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LookupNode {
    number: u64,
    head_ref_name: String,
    base_ref_name: String,
    viewer_latest_review: Option<ReviewNode>,
}

#[derive(Deserialize)]
struct ReviewNode {
    state: String,
}

impl PullRequestLookupData {
    /// What was found, by pull request number.
    pub fn into_pull_requests(self) -> HashMap<u64, LookedUpPullRequest> {
        self.found
            .into_pull_requests()
            .map(|node| {
                let looked_up = LookedUpPullRequest {
                    branches: PullRequestBranches {
                        head_ref: node.head_ref_name,
                        base_ref: node.base_ref_name,
                    },
                    viewer_review: node
                        .viewer_latest_review
                        .and_then(|review| ViewerReview::from_graphql_state(&review.state)),
                };
                (node.number, looked_up)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphql_response::GraphQlResponse;

    /// The query asks for each number's branches and review under its own alias.
    #[test]
    fn writes_one_aliased_field_per_number() {
        assert_eq!(
            pull_request_lookup_query(&[7, 12]),
            "query PullRequestLookup($owner: String!, $name: String!) {\n  repository(owner: $owner, name: $name) {\n    pr7: pullRequest(number: 7) { number headRefName baseRefName viewerLatestReview { state } }\n    pr12: pullRequest(number: 12) { number headRefName baseRefName viewerLatestReview { state } }\n  }\n}\n"
        );
    }

    /// The answer maps to branches and reviews by number; a null field is skipped, a pending review is none.
    #[test]
    fn reads_pull_requests_by_number() {
        let json = r#"{"data": {"repository": {
            "pr7": {"number": 7, "headRefName": "feat/x", "baseRefName": "main",
                    "viewerLatestReview": {"state": "APPROVED"}},
            "pr8": {"number": 8, "headRefName": "fix/y", "baseRefName": "main",
                    "viewerLatestReview": {"state": "PENDING"}},
            "pr9": null
        }}}"#;
        let response: GraphQlResponse<PullRequestLookupData> = serde_json::from_str(json).unwrap();
        let found = response.into_data().unwrap().into_pull_requests();
        assert_eq!(found.len(), 2);
        assert_eq!(found[&7].branches.head_ref, "feat/x");
        assert_eq!(found[&7].viewer_review, Some(ViewerReview::Approved));
        assert_eq!(found[&8].viewer_review, None);
    }
}
