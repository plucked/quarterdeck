use std::collections::HashMap;
use std::fmt::Write as _;

use serde::Deserialize;

use crate::{PullRequestBranches, ViewerReview};

/// The alias prefix of each looked-up pull request (`pr7`): GraphQL field
/// aliases must start with a letter.
const ALIAS_PREFIX: &str = "pr";

/// What a lookup found of one pull request.
pub(crate) struct LookedUpPullRequest {
    pub branches: PullRequestBranches,
    pub viewer_review: Option<ViewerReview>,
}

/// The GraphQL text looking up the branches and the viewer's latest review
/// of pull requests `numbers` of the repository in the `owner` and `name`
/// variables, one aliased `pullRequest(number:)` field each. The numbers
/// are integers, so writing them into the text cannot change the query.
pub(crate) fn pull_request_lookup_query(numbers: &[u64]) -> String {
    let mut fields = String::new();
    for number in numbers {
        // Writing to a String cannot fail.
        let _ = writeln!(
            fields,
            "    {ALIAS_PREFIX}{number}: pullRequest(number: {number}) {{ number headRefName baseRefName viewerLatestReview {{ state }} }}"
        );
    }
    format!(
        "query PullRequestLookup($owner: String!, $name: String!) {{\n  repository(owner: $owner, name: $name) {{\n{fields}  }}\n}}\n"
    )
}

/// The data of a [`pull_request_lookup_query`].
#[derive(Deserialize)]
pub(crate) struct PullRequestLookupData {
    /// `None` when the repository is gone or invisible to the token.
    repository: Option<HashMap<String, Option<LookupNode>>>,
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
        self.repository
            .into_iter()
            .flat_map(HashMap::into_values)
            .flatten()
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

    /// The query asks for each number under its own alias.
    #[test]
    fn writes_one_aliased_field_per_number() {
        let query = pull_request_lookup_query(&[7, 12]);
        assert!(query.contains(
            "pr7: pullRequest(number: 7) { number headRefName baseRefName viewerLatestReview { state } }"
        ));
        assert!(query.contains("pr12: pullRequest(number: 12)"));
        assert!(query.contains("repository(owner: $owner, name: $name)"));
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
