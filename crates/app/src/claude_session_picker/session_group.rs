use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// Sessions sharing one local repository, or the final group without repository metadata.
pub(super) struct SessionGroup {
    pub name: String,
    pub repository: Option<PathBuf>,
    pub sessions: Vec<usize>,
}

impl SessionGroup {
    /// Groups by full checkout identity and sorts repository names, keeping CLI order within each.
    pub fn for_repositories(repositories: impl IntoIterator<Item = Option<PathBuf>>) -> Vec<Self> {
        // A worktree's main-checkout path joins it to its siblings, never just its folder name.
        let mut members: BTreeMap<Option<PathBuf>, Vec<usize>> = BTreeMap::new();
        for (index, repository) in repositories.into_iter().enumerate() {
            members.entry(repository).or_default().push(index);
        }
        let mut groups: Vec<_> = members
            .into_iter()
            .map(|(repository, sessions)| {
                let name = match &repository {
                    Some(path) => path.file_name().map_or_else(
                        || path.display().to_string(),
                        |name| name.to_string_lossy().into_owned(),
                    ),
                    None => "Other sessions".to_owned(),
                };
                Self {
                    name,
                    repository,
                    sessions,
                }
            })
            .collect();

        // Identically named checkouts remain distinguishable without merging their sessions.
        let mut counts = HashMap::new();
        for group in &groups {
            *counts.entry(group.name.clone()).or_insert(0) += 1;
        }
        groups.sort_by_key(|group| {
            (
                group.repository.is_none(),
                group.name.to_lowercase(),
                group.repository.clone(),
            )
        });
        for group in &mut groups {
            if counts[&group.name] > 1
                && let Some(repository) = &group.repository
            {
                group.name = repository.display().to_string();
            }
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Interleaved worktrees join one group; independent same-name clones and non-Git sessions do not.
    #[test]
    fn groups_by_repository_identity_in_name_order() {
        let groups = SessionGroup::for_repositories([
            Some("/projects/zeta".into()),
            Some("/projects/alpha".into()),
            None,
            Some("/projects/zeta".into()),
            Some("/other/alpha".into()),
        ]);
        assert_eq!(
            groups
                .iter()
                .map(|group| group.name.as_str())
                .collect::<Vec<_>>(),
            ["/other/alpha", "/projects/alpha", "zeta", "Other sessions"]
        );
        assert_eq!(
            groups
                .iter()
                .map(|group| group.sessions.as_slice())
                .collect::<Vec<_>>(),
            [&[4][..], &[1], &[0, 3], &[2]]
        );
    }
}
