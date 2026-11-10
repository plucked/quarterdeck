use std::collections::HashMap;

use serde::Deserialize;

use crate::aliased_pull_requests_query::{AliasedPullRequestsData, AliasedPullRequestsQuery};
use crate::status_checks_query::CONFLICTING;

/// The question whether stack layers can merge into their bases.
const LAYER_CONFLICTS_QUERY: AliasedPullRequestsQuery = AliasedPullRequestsQuery {
    operation: "LayerConflicts",
    alias_prefix: "layer",
    selection: "number mergeable baseRefName",
};

/// The GraphQL text asking whether pull requests `numbers` of the
/// repository in the `owner` and `name` variables can merge into their
/// base, one aliased `pullRequest(number:)` field each.
pub(crate) fn layer_conflicts_query(numbers: &[u64]) -> String {
    LAYER_CONFLICTS_QUERY.text(numbers)
}

/// The data of a [`layer_conflicts_query`].
#[derive(Deserialize)]
#[serde(transparent)]
pub(crate) struct LayerConflictsData {
    layers: AliasedPullRequestsData<LayerNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LayerNode {
    number: u64,
    /// `MERGEABLE`, `CONFLICTING` or `UNKNOWN` (not worked out yet).
    mergeable: String,
    base_ref_name: String,
}

/// A layer GitHub reports as conflicting, with the branch it conflicts with.
pub(crate) struct ConflictingLayer {
    pub number: u64,
    pub base_ref: String,
}

impl LayerConflictsData {
    /// The first of `numbers` (in their order) that conflicts with its base.
    pub fn first_conflicting(self, numbers: &[u64]) -> Option<ConflictingLayer> {
        let conflicting: HashMap<u64, String> = self
            .layers
            .into_pull_requests()
            .filter(|node| node.mergeable == CONFLICTING)
            .map(|node| (node.number, node.base_ref_name))
            .collect();
        numbers.iter().find_map(|number| {
            conflicting.get(number).map(|base_ref| ConflictingLayer {
                number: *number,
                base_ref: base_ref.clone(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The query asks about each layer under its own alias.
    #[test]
    fn query_aliases_each_layer() {
        assert_eq!(
            layer_conflicts_query(&[36410, 36454]),
            "query LayerConflicts($owner: String!, $name: String!) {\n  repository(owner: $owner, name: $name) {\n    layer36410: pullRequest(number: 36410) { number mergeable baseRefName }\n    layer36454: pullRequest(number: 36454) { number mergeable baseRefName }\n  }\n}\n"
        );
    }

    /// The lowest conflicting layer wins; unknown ones do not count.
    #[test]
    fn the_lowest_conflicting_layer_is_found() {
        let data: LayerConflictsData = serde_json::from_str(
            r#"{"repository":{
              "layer36454":{"number":36454,"mergeable":"CONFLICTING","baseRefName":"b086"},
              "layer36410":{"number":36410,"mergeable":"CONFLICTING","baseRefName":"develop"},
              "layer36460":{"number":36460,"mergeable":"UNKNOWN","baseRefName":"b092"}}}"#,
        )
        .unwrap();
        let found = data.first_conflicting(&[36410, 36454, 36460]).unwrap();
        assert_eq!(found.number, 36410);
        assert_eq!(found.base_ref, "develop");
    }
}
