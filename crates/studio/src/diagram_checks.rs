//! Checks a graph and a sequence share: the agent's ids to indexes, and
//! labels that must be written.

use std::collections::HashMap;

/// Each id with its position; an error naming the first repeated one.
pub(crate) fn index_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    kind: &str,
) -> Result<HashMap<String, usize>, String> {
    let mut indexes = HashMap::new();
    for (index, id) in ids.enumerate() {
        if indexes.insert(id.to_owned(), index).is_some() {
            return Err(format!("the {kind} id “{id}” is used twice"));
        }
    }
    Ok(indexes)
}

/// An error naming what lacks a label, when `label` is blank: an unnamed
/// box or arrow explains nothing.
pub(crate) fn require_label(label: &str, what: impl FnOnce() -> String) -> Result<(), String> {
    if label.trim().is_empty() {
        return Err(format!("{} has no label", what()));
    }
    Ok(())
}
