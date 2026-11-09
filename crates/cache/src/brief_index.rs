use std::collections::{HashMap, HashSet};

use qd_github::RepoRef;

/// Which pull requests have a stored brief, for marking them in the queue:
/// by repository and number, which the search and pulls APIs agree on
/// (their ids differ). A combined brief counts for the layer its range ends
/// at, the pull request it was written from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BriefIndex {
    /// The numbers with a brief, by owner, then repository name, so a
    /// lookup borrows the `RepoRef`'s strings (the queue asks for every row
    /// it draws, every frame).
    briefed: HashMap<String, HashMap<String, HashSet<u64>>>,
}

impl BriefIndex {
    /// Notes that pull request `number` of `repository` (`owner/name`, as
    /// stored with the brief) has a brief.
    pub fn insert(&mut self, repository: &str, number: u64) {
        let (owner, name) = repository.split_once('/').unwrap_or((repository, ""));
        self.briefed
            .entry(owner.to_owned())
            .or_default()
            .entry(name.to_owned())
            .or_default()
            .insert(number);
    }

    /// Whether pull request `number` of `repo` has a brief.
    pub fn has_brief(&self, repo: &RepoRef, number: u64) -> bool {
        self.briefed
            .get(&repo.owner)
            .and_then(|names| names.get(&repo.name))
            .is_some_and(|numbers| numbers.contains(&number))
    }
}
