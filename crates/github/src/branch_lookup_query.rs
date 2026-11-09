use std::collections::HashMap;
use std::fmt::Write as _;

use serde::Deserialize;

use crate::PullRequestBranches;

/// The alias prefix of each looked-up pull request (`pr7`): GraphQL field
/// aliases must start with a letter.
const ALIAS_PREFIX: &str = "pr";

/// The GraphQL text looking up the branches of pull requests `numbers` of
/// the repository in the `owner` and `name` variables, one aliased
/// `pullRequest(number:)` field each. The numbers are integers, so writing
/// them into the text cannot change the query.
pub(crate) fn branch_lookup_query(numbers: &[u64]) -> String {
    let mut fields = String::new();
    for number in numbers {
        // Writing to a String cannot fail.
        let _ = writeln!(
            fields,
            "    {ALIAS_PREFIX}{number}: pullRequest(number: {number}) {{ number headRefName baseRefName }}"
        );
    }
    format!(
        "query BranchLookup($owner: String!, $name: String!) {{\n  repository(owner: $owner, name: $name) {{\n{fields}  }}\n}}\n"
    )
}

/// The data of a [`branch_lookup_query`].
#[derive(Deserialize)]
pub(crate) struct BranchLookupData {
    /// `None` when the repository is gone or invisible to the token.
    repository: Option<HashMap<String, Option<BranchNode>>>,
}

/// One looked-up pull request.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BranchNode {
    number: u64,
    head_ref_name: String,
    base_ref_name: String,
}

impl BranchLookupData {
    /// The branches found, by pull request number.
    pub fn into_branches(self) -> HashMap<u64, PullRequestBranches> {
        self.repository
            .into_iter()
            .flat_map(HashMap::into_values)
            .flatten()
            .map(|node| {
                (
                    node.number,
                    PullRequestBranches {
                        head_ref: node.head_ref_name,
                        base_ref: node.base_ref_name,
                    },
                )
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
        let query = branch_lookup_query(&[7, 12]);
        assert!(query.contains("pr7: pullRequest(number: 7) { number headRefName baseRefName }"));
        assert!(query.contains("pr12: pullRequest(number: 12)"));
        assert!(query.contains("repository(owner: $owner, name: $name)"));
    }

    /// The answer maps to branches by number; a null field is skipped.
    #[test]
    fn reads_branches_by_number() {
        let json = r#"{"data": {"repository": {
            "pr7": {"number": 7, "headRefName": "feat/x", "baseRefName": "main"},
            "pr9": null
        }}}"#;
        let response: GraphQlResponse<BranchLookupData> = serde_json::from_str(json).unwrap();
        let branches = response.into_data().unwrap().into_branches();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[&7].head_ref, "feat/x");
        assert_eq!(branches[&7].base_ref, "main");
    }
}
