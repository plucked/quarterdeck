use std::collections::HashSet;

use qd_github::RepoRef;

use super::stack_key::StackKey;

/// The sidebar groups the user folded: repositories and stacks. Kept while
/// the app runs; everything starts expanded at launch.
#[derive(Debug, Default)]
pub struct CollapsedGroups {
    pub repositories: HashSet<RepoRef>,
    pub stacks: HashSet<StackKey>,
}

impl CollapsedGroups {
    /// Folds repository `repo`'s group, or unfolds it.
    pub fn toggle_repository(&mut self, repo: &RepoRef) {
        if !self.repositories.remove(repo) {
            self.repositories.insert(repo.clone());
        }
    }

    /// Folds stack `key`'s group, or unfolds it.
    pub fn toggle_stack(&mut self, key: &StackKey) {
        if !self.stacks.remove(key) {
            self.stacks.insert(key.clone());
        }
    }
}
