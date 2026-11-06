use serde::Deserialize;

use crate::{ListedStackMembership, RepoRef, StackMembership};

/// One GraphQL query for the stack membership of every pull request in the
/// viewer's lists, run with the same search texts the REST lists use (as
/// the `authored`, `reviewRequested` and `others` variables; `others` only
/// when `includeOthers`). Costs one point of the GraphQL budget.
pub(crate) const STACK_MEMBERSHIP_QUERY: &str = r"
query StackMemberships(
  $authored: String!, $reviewRequested: String!, $others: String!, $includeOthers: Boolean!
) {
  authored: search(query: $authored, type: ISSUE, first: 100) {
    nodes { ...membership }
  }
  reviewRequested: search(query: $reviewRequested, type: ISSUE, first: 100) {
    nodes { ...membership }
  }
  others: search(query: $others, type: ISSUE, first: 100) @include(if: $includeOthers) {
    nodes { ...membership }
  }
}
fragment membership on PullRequest {
  number
  repository { name owner { login } }
  stack { number size baseRefName }
  stackEntry { position }
}
";

/// The data of [`STACK_MEMBERSHIP_QUERY`].
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StackMembershipData {
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

impl StackMembershipData {
    /// The memberships of stacked pull requests in the lists, each once.
    pub fn into_memberships(self) -> Vec<ListedStackMembership> {
        let data = self;
        let mut memberships: Vec<ListedStackMembership> = Vec::new();
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
            if !memberships.contains(&listed) {
                memberships.push(listed);
            }
        }
        memberships
    }
}

impl SearchNode {
    /// The membership of a stacked pull request; `None` for others.
    fn into_listed(self) -> Option<ListedStackMembership> {
        let stack = self.stack?;
        let repository = self.repository?;
        Some(ListedStackMembership {
            repo: RepoRef {
                owner: repository.owner.login,
                name: repository.name,
            },
            number: self.number?,
            membership: StackMembership {
                number: stack.number,
                position: self.stack_entry?.position,
                size: stack.size,
                base_ref: stack.base_ref_name,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphql_response::GraphQlResponse;

    /// Stacked pull requests are kept once each; standalone ones and non-pull-request results are skipped.
    #[test]
    fn reads_memberships_of_both_lists() {
        let json = r#"{"data": {
            "authored": {"nodes": [
                {"number": 7, "repository": {"name": "b", "owner": {"login": "a"}},
                 "stack": {"number": 99, "size": 3, "baseRefName": "main"}, "stackEntry": {"position": 2}},
                {"number": 8, "repository": {"name": "b", "owner": {"login": "a"}}, "stack": null, "stackEntry": null},
                {}
            ]},
            "reviewRequested": {"nodes": [
                {"number": 7, "repository": {"name": "b", "owner": {"login": "a"}},
                 "stack": {"number": 99, "size": 3, "baseRefName": "main"}, "stackEntry": {"position": 2}}
            ]}
        }}"#;
        let response: GraphQlResponse<StackMembershipData> = serde_json::from_str(json).unwrap();
        let memberships = response.into_data().unwrap().into_memberships();
        assert_eq!(memberships.len(), 1);
        assert_eq!(memberships[0].number, 7);
        assert_eq!(memberships[0].membership.position, 2);
        assert_eq!(memberships[0].membership.base_ref, "main");
    }

    /// A GraphQL failure without data becomes its messages.
    #[test]
    fn reports_graphql_errors() {
        let json = r#"{"data": null, "errors": [{"message": "Something went wrong"}]}"#;
        let response: GraphQlResponse<StackMembershipData> = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.into_data().err().as_deref(),
            Some("Something went wrong")
        );
    }
}
