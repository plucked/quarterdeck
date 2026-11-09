use serde::Deserialize;

use crate::{ListedPullRequestDetails, PullRequestBranches, RepoRef, StackMembership};

/// One GraphQL query for the branches and stack membership of every pull
/// request in the viewer's lists, run with the same search texts the REST
/// lists use (as the `authored`, `reviewRequested` and `others` variables;
/// `others` only when `includeOthers`). Costs one point of the GraphQL budget.
pub(crate) const LISTED_DETAILS_QUERY: &str = r"
query ListedDetails(
  $authored: String!, $reviewRequested: String!, $others: String!, $includeOthers: Boolean!
) {
  authored: search(query: $authored, type: ISSUE, first: 100) {
    nodes { ...details }
  }
  reviewRequested: search(query: $reviewRequested, type: ISSUE, first: 100) {
    nodes { ...details }
  }
  others: search(query: $others, type: ISSUE, first: 100) @include(if: $includeOthers) {
    nodes { ...details }
  }
}
fragment details on PullRequest {
  number
  repository { name owner { login } }
  headRefName
  baseRefName
  stack { number size baseRefName }
  stackEntry { position }
}
";

/// The data of [`LISTED_DETAILS_QUERY`].
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListedDetailsData {
    authored: SearchNodes,
    review_requested: SearchNodes,
    /// Absent unless the query included it.
    #[serde(default)]
    others: Option<SearchNodes>,
}

#[derive(Deserialize)]
struct SearchNodes {
    nodes: Vec<SearchNode>,
}

/// A search result; empty (every field absent) for results that are not
/// pull requests, which the fragment does not cover.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchNode {
    number: Option<u64>,
    repository: Option<Repository>,
    head_ref_name: Option<String>,
    base_ref_name: Option<String>,
    stack: Option<Stack>,
    stack_entry: Option<StackEntry>,
}

#[derive(Deserialize)]
struct Repository {
    name: String,
    owner: Owner,
}

#[derive(Deserialize)]
struct Owner {
    login: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stack {
    number: u64,
    size: u64,
    base_ref_name: String,
}

#[derive(Deserialize)]
struct StackEntry {
    position: u64,
}

impl ListedDetailsData {
    /// The details of every pull request in the lists, each once.
    pub fn into_details(self) -> Vec<ListedPullRequestDetails> {
        let data = self;
        let mut details: Vec<ListedPullRequestDetails> = Vec::new();
        for node in data
            .authored
            .nodes
            .into_iter()
            .chain(data.review_requested.nodes)
            .chain(data.others.into_iter().flat_map(|others| others.nodes))
        {
            let Some(listed) = node.into_listed() else {
                continue;
            };
            if !details.contains(&listed) {
                details.push(listed);
            }
        }
        details
    }
}

impl SearchNode {
    /// The details of a pull request; `None` for other results.
    fn into_listed(self) -> Option<ListedPullRequestDetails> {
        let repository = self.repository?;
        let stack = match (self.stack, self.stack_entry) {
            (Some(stack), Some(entry)) => Some(StackMembership {
                number: stack.number,
                position: entry.position,
                size: stack.size,
                base_ref: stack.base_ref_name,
            }),
            _ => None,
        };
        Some(ListedPullRequestDetails {
            repo: RepoRef {
                owner: repository.owner.login,
                name: repository.name,
            },
            number: self.number?,
            branches: PullRequestBranches {
                head_ref: self.head_ref_name?,
                base_ref: self.base_ref_name?,
            },
            stack,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphql_response::GraphQlResponse;

    /// Every pull request is kept once with its branches; only stacked ones carry a membership, and non-pull-request results are skipped.
    #[test]
    fn reads_details_of_both_lists() {
        let json = r#"{"data": {
            "authored": {"nodes": [
                {"number": 7, "repository": {"name": "b", "owner": {"login": "a"}},
                 "headRefName": "feat/x", "baseRefName": "main",
                 "stack": {"number": 99, "size": 3, "baseRefName": "main"}, "stackEntry": {"position": 2}},
                {"number": 8, "repository": {"name": "b", "owner": {"login": "a"}},
                 "headRefName": "fix/y", "baseRefName": "release/1",
                 "stack": null, "stackEntry": null},
                {}
            ]},
            "reviewRequested": {"nodes": [
                {"number": 7, "repository": {"name": "b", "owner": {"login": "a"}},
                 "headRefName": "feat/x", "baseRefName": "main",
                 "stack": {"number": 99, "size": 3, "baseRefName": "main"}, "stackEntry": {"position": 2}}
            ]}
        }}"#;
        let response: GraphQlResponse<ListedDetailsData> = serde_json::from_str(json).unwrap();
        let details = response.into_data().unwrap().into_details();
        assert_eq!(details.len(), 2);
        assert_eq!(details[0].number, 7);
        assert_eq!(details[0].branches.head_ref, "feat/x");
        let stack = details[0].stack.as_ref().unwrap();
        assert_eq!(stack.position, 2);
        assert_eq!(stack.base_ref, "main");
        assert_eq!(details[1].branches.base_ref, "release/1");
        assert_eq!(details[1].stack, None);
    }

    /// A GraphQL failure without data becomes its messages.
    #[test]
    fn reports_graphql_errors() {
        let json = r#"{"data": null, "errors": [{"message": "Something went wrong"}]}"#;
        let response: GraphQlResponse<ListedDetailsData> = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.into_data().err().as_deref(),
            Some("Something went wrong")
        );
    }
}
