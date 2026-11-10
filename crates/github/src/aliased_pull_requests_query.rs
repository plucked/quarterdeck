use std::collections::HashMap;
use std::fmt::Write as _;

use serde::Deserialize;

/// A GraphQL query reading chosen pull requests of the repository in the
/// `owner` and `name` variables, one aliased `pullRequest(number:)` field
/// each, so one request answers for all of them.
pub(crate) struct AliasedPullRequestsQuery {
    /// The query's operation name (`PullRequestLookup`).
    pub operation: &'static str,
    /// What each alias starts with (`pr` → `pr7`): GraphQL field aliases
    /// must start with a letter.
    pub alias_prefix: &'static str,
    /// The fields read of each pull request (`number mergeable baseRefName`).
    pub selection: &'static str,
}

impl AliasedPullRequestsQuery {
    /// The query text for pull requests `numbers`. The numbers are
    /// integers, so writing them into the text cannot change the query.
    pub fn text(&self, numbers: &[u64]) -> String {
        let Self {
            operation,
            alias_prefix,
            selection,
        } = self;
        let mut fields = String::new();
        for number in numbers {
            // Writing to a String cannot fail.
            let _ = writeln!(
                fields,
                "    {alias_prefix}{number}: pullRequest(number: {number}) {{ {selection} }}"
            );
        }
        format!(
            "query {operation}($owner: String!, $name: String!) {{\n  repository(owner: $owner, name: $name) {{\n{fields}  }}\n}}\n"
        )
    }
}

/// The data of an [`AliasedPullRequestsQuery`]: each alias's pull request,
/// shaped `T` by its selection.
#[derive(Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub(crate) struct AliasedPullRequestsData<T> {
    /// `None` when the repository is gone or invisible to the token; an
    /// alias is `null` when GitHub has no such pull request.
    repository: Option<HashMap<String, Option<T>>>,
}

impl<T> AliasedPullRequestsData<T> {
    /// Every pull request found, in no particular order.
    pub fn into_pull_requests(self) -> impl Iterator<Item = T> {
        self.repository
            .into_iter()
            .flat_map(HashMap::into_values)
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text names the operation and asks for each number under its own alias.
    #[test]
    fn writes_one_aliased_field_per_number() {
        let query = AliasedPullRequestsQuery {
            operation: "Sample",
            alias_prefix: "pr",
            selection: "number title",
        };
        assert_eq!(
            query.text(&[7, 12]),
            "query Sample($owner: String!, $name: String!) {\n  repository(owner: $owner, name: $name) {\n    pr7: pullRequest(number: 7) { number title }\n    pr12: pullRequest(number: 12) { number title }\n  }\n}\n"
        );
    }

    /// Null aliases and a missing repository read as nothing found.
    #[test]
    fn skips_what_github_did_not_find() {
        let data: AliasedPullRequestsData<u64> =
            serde_json::from_str(r#"{"repository":{"pr7":7,"pr8":null}}"#).unwrap();
        assert_eq!(data.into_pull_requests().collect::<Vec<_>>(), [7]);
        let gone: AliasedPullRequestsData<u64> =
            serde_json::from_str(r#"{"repository":null}"#).unwrap();
        assert_eq!(gone.into_pull_requests().count(), 0);
    }
}
