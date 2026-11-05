use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result};
use qc_github::{PullRequest, PullRequestFile};
use rusqlite::{Connection, OptionalExtension, params};

use crate::PullRequestListKey;
use crate::database_connection::{default_database_path, open_database};

/// The SQLite connection holding cached GitHub data. Every write runs in one
/// statement or one transaction, so the database is consistent after any failure.
pub struct GitHubCache {
    connection: Connection,
}

impl GitHubCache {
    /// Opens (creating if needed) the default database file.
    pub fn open_default() -> Result<Self> {
        Self::open(&default_database_path()?)
    }

    /// Opens (creating if needed) the database at `path` and migrates it.
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            connection: open_database(path)?,
        })
    }

    /// A fresh, migrated database that lives only as long as the value.
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        Ok(Self {
            connection: crate::database_connection::open_in_memory_database()?,
        })
    }

    /// Replaces the members of a list with `pull_requests`, in that order,
    /// upserting the pull requests themselves.
    pub fn replace_list(
        &mut self,
        list_key: PullRequestListKey,
        pull_requests: &[PullRequest],
    ) -> Result<()> {
        let fetched_at = unix_now();
        let transaction = self.connection.transaction()?;

        // Drop the old membership; the pull request rows stay for other lists.
        transaction.execute(
            "DELETE FROM pr_lists WHERE list_key = ?1",
            [list_key.as_str()],
        )?;

        // Upsert each pull request and record its position in the list.
        for (position, pull_request) in pull_requests.iter().enumerate() {
            let id = sql_integer(pull_request.id)?;
            transaction.execute(
                "INSERT INTO pull_requests (id, repo, number, updated_at, fetched_at, data)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (id) DO UPDATE SET
                    updated_at = excluded.updated_at,
                    fetched_at = excluded.fetched_at,
                    data       = excluded.data",
                params![
                    id,
                    pull_request.repo.full_name(),
                    sql_integer(pull_request.number)?,
                    pull_request.updated_at,
                    fetched_at,
                    serde_json::to_string(pull_request)?
                ],
            )?;
            transaction.execute(
                "INSERT INTO pr_lists (list_key, pr_id, position) VALUES (?1, ?2, ?3)",
                params![list_key.as_str(), id, i64::try_from(position)?],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// The pull requests of a list, in stored order.
    pub fn list(&self, list_key: PullRequestListKey) -> Result<Vec<PullRequest>> {
        let mut statement = self.connection.prepare(
            "SELECT p.data FROM pr_lists l
             JOIN pull_requests p ON p.id = l.pr_id
             WHERE l.list_key = ?1 ORDER BY l.position",
        )?;
        let rows = statement.query_map([list_key.as_str()], |row| row.get::<_, String>(0))?;
        rows.map(|data| Ok(serde_json::from_str(&data?)?)).collect()
    }

    /// Stores the changed files of `pull_request`, tagged with its `updated_at`.
    /// The pull request must already be cached.
    pub fn put_files(&self, pull_request: &PullRequest, files: &[PullRequestFile]) -> Result<()> {
        self.connection.execute(
            "INSERT INTO pr_files (pr_id, updated_at, data) VALUES (?1, ?2, ?3)
             ON CONFLICT (pr_id) DO UPDATE SET updated_at = excluded.updated_at, data = excluded.data",
            params![
                sql_integer(pull_request.id)?,
                pull_request.updated_at,
                serde_json::to_string(files)?
            ],
        )?;
        Ok(())
    }

    /// The cached changed files, or `None` when missing or fetched for an older
    /// `updated_at` than `pull_request` has.
    pub fn files(&self, pull_request: &PullRequest) -> Result<Option<Vec<PullRequestFile>>> {
        let data: Option<String> = self
            .connection
            .query_row(
                "SELECT data FROM pr_files WHERE pr_id = ?1 AND updated_at = ?2",
                params![sql_integer(pull_request.id)?, pull_request.updated_at],
                |row| row.get(0),
            )
            .optional()?;
        data.map(|data| Ok(serde_json::from_str(&data)?))
            .transpose()
    }
}

/// Converts a GitHub id or number to SQLite's signed integer, failing instead
/// of wrapping when it does not fit.
fn sql_integer(value: u64) -> Result<i64> {
    i64::try_from(value).context("value exceeds SQLite's integer range")
}

/// Seconds since the Unix epoch; 0 if the clock is before it.
fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qc_github::{RepoRef, User};

    const LIST: PullRequestListKey = PullRequestListKey::REVIEW_REQUESTED;

    fn pull_request(id: u64, updated_at: &str) -> PullRequest {
        PullRequest {
            id,
            number: id,
            repo: RepoRef {
                owner: "a".into(),
                name: "b".into(),
            },
            title: format!("PR {id}"),
            body: None,
            author: User {
                login: "octo".into(),
                avatar_url: String::new(),
            },
            html_url: String::new(),
            draft: false,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: updated_at.into(),
        }
    }

    /// A list replacement keeps the new order and drops members not in it.
    #[test]
    fn replace_list_keeps_order_and_drops_old_members() {
        let mut cache = GitHubCache::open_in_memory().unwrap();
        cache
            .replace_list(LIST, &[pull_request(1, "t"), pull_request(2, "t")])
            .unwrap();
        cache
            .replace_list(LIST, &[pull_request(3, "t"), pull_request(1, "t2")])
            .unwrap();
        let listed = cache.list(LIST).unwrap();
        let ids: Vec<_> = listed.iter().map(|listed| listed.id).collect();
        assert_eq!(ids, [3, 1]);
        assert_eq!(listed[1].updated_at, "t2");
    }

    /// Cached files are ignored once the pull request has a newer `updated_at`.
    #[test]
    fn files_invalidate_on_pull_request_update() {
        // Set up: one cached pull request with one cached file.
        let mut cache = GitHubCache::open_in_memory().unwrap();
        let old = pull_request(1, "t1");
        cache
            .replace_list(LIST, std::slice::from_ref(&old))
            .unwrap();
        let file = PullRequestFile {
            filename: "a.rs".into(),
            status: "modified".into(),
            additions: 1,
            deletions: 0,
            patch: None,
        };
        cache.put_files(&old, &[file]).unwrap();

        // Expect: the same version hits, a newer version misses.
        assert_eq!(cache.files(&old).unwrap().unwrap().len(), 1);
        assert!(cache.files(&pull_request(1, "t2")).unwrap().is_none());
    }
}
