use anyhow::Result;
use qd_github::RepoRef;
use rusqlite::params;

use crate::UiStateStore;

impl UiStateStore {
    /// The repositories the Others tab lists, in the order they were added.
    pub fn watched_repositories(&self) -> Result<Vec<RepoRef>> {
        let mut statement = self
            .connection()
            .prepare("SELECT repo_owner, repo_name FROM watched_repositories ORDER BY position")?;
        let repositories = statement
            .query_map([], |row| {
                Ok(RepoRef {
                    owner: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(repositories)
    }

    /// Replaces the watched repositories with `repositories`, in that order.
    pub fn put_watched_repositories(&self, repositories: &[RepoRef]) -> Result<()> {
        let transaction = self.connection().unchecked_transaction()?;
        transaction.execute("DELETE FROM watched_repositories", [])?;
        for (position, repo) in repositories.iter().enumerate() {
            transaction.execute(
                "INSERT INTO watched_repositories (repo_owner, repo_name, position) VALUES (?1, ?2, ?3)",
                params![repo.owner, repo.name, i64::try_from(position)?],
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
            owner: "o".into(),
            name: name.into(),
        }
    }

    /// The repositories come back in the order they were saved.
    #[test]
    fn keeps_the_order() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert_eq!(store.watched_repositories().unwrap(), Vec::<RepoRef>::new());
        let repositories = [repo("web"), repo("api")];
        store.put_watched_repositories(&repositories).unwrap();
        assert_eq!(store.watched_repositories().unwrap(), repositories);
        store.put_watched_repositories(&[repo("api")]).unwrap();
        assert_eq!(store.watched_repositories().unwrap(), [repo("api")]);
    }
}
