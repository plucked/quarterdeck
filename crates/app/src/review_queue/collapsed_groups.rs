use std::collections::HashSet;

use qd_cache::{FoldedGroups, FoldedStack};
use qd_github::RepoRef;

use super::stack_key::StackKey;

/// The sidebar groups the user folded: repositories and stacks. Saved as
/// UI state ([`FoldedGroups`]), so they stay folded across launches.
#[derive(Debug, Default)]
pub struct CollapsedGroups {
    pub repositories: HashSet<RepoRef>,
    pub stacks: HashSet<StackKey>,
}

impl CollapsedGroups {
    /// The groups folded before the last relaunch.
    pub fn from_saved(saved: FoldedGroups) -> Self {
        Self {
            repositories: saved.repositories.into_iter().collect(),
            stacks: saved
                .stacks
                .into_iter()
                .map(|stack| StackKey {
                    repo: stack.repo,
                    number: stack.number,
                })
                .collect(),
        }
    }

    /// The groups to save.
    pub fn to_saved(&self) -> FoldedGroups {
        FoldedGroups {
            repositories: self.repositories.iter().cloned().collect(),
            stacks: self
                .stacks
                .iter()
                .map(|key| FoldedStack {
                    repo: key.repo.clone(),
                    number: key.number,
                })
                .collect(),
        }
    }

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
