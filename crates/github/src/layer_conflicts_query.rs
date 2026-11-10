use std::collections::HashMap;
use std::fmt::Write as _;

use serde::Deserialize;

/// The alias prefix of each asked-about layer (`layer7`): GraphQL field
/// aliases must start with a letter.
const ALIAS_PREFIX: &str = "layer";

/// The GraphQL text asking whether pull requests `numbers` of the
/// repository in the `owner` and `name` variables can merge into their
/// base, one aliased `pullRequest(number:)` field each. The numbers are
/// integers, so writing them into the text cannot change the query.
pub(crate) fn layer_conflicts_query(numbers: &[u64]) -> String {
    let mut fields = String::new();
    for number in numbers {
        // Writing to a String cannot fail.
        let _ = writeln!(
            fields,
            "    {ALIAS_PREFIX}{number}: pullRequest(number: {number}) {{ number mergeable baseRefName }}"
        );
    }
    format!(
        "query LayerConflicts($owner: String!, $name: String!) {{\n  repository(owner: $owner, name: $name) {{\n{fields}  }}\n}}\n"
    )
}

/// The data of a [`layer_conflicts_query`].
#[derive(Deserialize)]
pub(crate) struct LayerConflictsData {
    /// `None` when the repository is gone or invisible to the token.
    repository: Option<HashMap<String, Option<LayerNode>>>,
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
            .repository
            .into_iter()
            .flat_map(HashMap::into_values)
            .flatten()
            .filter(|node| node.mergeable == "CONFLICTING")
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
        let query = layer_conflicts_query(&[36410, 36454]);
        assert!(query.contains("layer36410: pullRequest(number: 36410)"));
        assert!(query.contains("layer36454: pullRequest(number: 36454)"));
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
