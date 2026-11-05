use anyhow::Result;
use qd_github::RepoRef;
use rusqlite::params;

use crate::sql_integers::{read_u64, sql_integer};
use crate::{FoldedGroups, FoldedStack, UiStateStore};

impl UiStateStore {
    /// The folded sidebar groups, as last saved; none when nothing was folded.
    pub fn folded_groups(&self) -> Result<FoldedGroups> {
        let mut repositories = self.connection().prepare(
            "SELECT repo_owner, repo_name FROM folded_repositories ORDER BY repo_owner, repo_name",
        )?;
        let repositories = repositories
            .query_map([], |row| {
                Ok(RepoRef {
                    owner: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut stacks = self.connection().prepare(
            "SELECT repo_owner, repo_name, stack_number FROM folded_stacks
             ORDER BY repo_owner, repo_name, stack_number",
        )?;
        let stacks = stacks
            .query_map([], |row| {
                Ok(FoldedStack {
                    repo: RepoRef {
                        owner: row.get(0)?,
                        name: row.get(1)?,
                    },
                    number: read_u64(row, 2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(FoldedGroups {
            repositories,
            stacks,
        })
    }

    /// Replaces the folded sidebar groups with `groups`, in one transaction.
    pub fn put_folded_groups(&self, groups: &FoldedGroups) -> Result<()> {
        let transaction = self.connection().unchecked_transaction()?;
        transaction.execute("DELETE FROM folded_repositories", [])?;
        transaction.execute("DELETE FROM folded_stacks", [])?;
        for repo in &groups.repositories {
            transaction.execute(
                "INSERT INTO folded_repositories (repo_owner, repo_name) VALUES (?1, ?2)",
                params![repo.owner, repo.name],
            )?;
        }
        for stack in &groups.stacks {
            transaction.execute(
                "INSERT INTO folded_stacks (repo_owner, repo_name, stack_number) VALUES (?1, ?2, ?3)",
                params![stack.repo.owner, stack.repo.name, sql_integer(stack.number)?],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(name: &str) -> RepoRef {
        RepoRef {
            owner: "klanggames".into(),
            name: name.into(),
        }
    }

    /// Folded groups round-trip, and each save replaces the last one.
    #[test]
    fn folded_groups_round_trip() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert_eq!(store.folded_groups().unwrap(), FoldedGroups::default());

        let folded = FoldedGroups {
            repositories: vec![repo("seed"), repo("web")],
            stacks: vec![FoldedStack {
                repo: repo("seed"),
                number: 35_799,
            }],
        };
        store.put_folded_groups(&folded).unwrap();
        assert_eq!(store.folded_groups().unwrap(), folded);

        let fewer = FoldedGroups {
            repositories: vec![repo("web")],
            stacks: Vec::new(),
        };
        store.put_folded_groups(&fewer).unwrap();
        assert_eq!(store.folded_groups().unwrap(), fewer);
    }
}
