use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result};
use qc_github::{PullRequest, PullRequestFile, RepoRef, User};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::PullRequestListKey;
use crate::database_connection::{default_database_path, open_database};

/// The SQLite connection holding cached GitHub data, one typed column per
/// field. Every write runs in one statement or one transaction, so the
/// database is consistent after any failure.
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
                "INSERT INTO pull_requests (
                    id, repo_owner, repo_name, number, title, body, author_login,
                    author_avatar_url, html_url, draft, created_at, updated_at, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT (id) DO UPDATE SET
                    repo_owner        = excluded.repo_owner,
                    repo_name         = excluded.repo_name,
                    number            = excluded.number,
                    title             = excluded.title,
                    body              = excluded.body,
                    author_login      = excluded.author_login,
                    author_avatar_url = excluded.author_avatar_url,
                    html_url          = excluded.html_url,
                    draft             = excluded.draft,
                    created_at        = excluded.created_at,
                    updated_at        = excluded.updated_at,
                    fetched_at        = excluded.fetched_at",
                params![
                    id,
                    pull_request.repo.owner,
                    pull_request.repo.name,
                    sql_integer(pull_request.number)?,
                    pull_request.title,
                    pull_request.body,
                    pull_request.author.login,
                    pull_request.author.avatar_url,
                    pull_request.html_url,
                    pull_request.draft,
                    pull_request.created_at,
                    pull_request.updated_at,
                    fetched_at
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
            "SELECT p.id, p.repo_owner, p.repo_name, p.number, p.title, p.body,
                    p.author_login, p.author_avatar_url, p.html_url, p.draft,
                    p.created_at, p.updated_at
             FROM pr_lists l
             JOIN pull_requests p ON p.id = l.pr_id
             WHERE l.list_key = ?1 ORDER BY l.position",
        )?;
        let rows = statement.query_map([list_key.as_str()], pull_request_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Stores the changed files of `pull_request`, tagged with its
    /// `updated_at` and replacing any stored before. The pull request must
    /// already be cached.
    pub fn put_files(
        &mut self,
        pull_request: &PullRequest,
        files: &[PullRequestFile],
    ) -> Result<()> {
        let pr_id = sql_integer(pull_request.id)?;
        let transaction = self.connection.transaction()?;

        // Tag the set with the version it belongs to, and clear the old files.
        transaction.execute(
            "INSERT INTO pr_file_sets (pr_id, updated_at) VALUES (?1, ?2)
             ON CONFLICT (pr_id) DO UPDATE SET updated_at = excluded.updated_at",
            params![pr_id, pull_request.updated_at],
        )?;
        transaction.execute("DELETE FROM pr_files WHERE pr_id = ?1", [pr_id])?;

        // Insert the files in GitHub's order.
        for (position, file) in files.iter().enumerate() {
            transaction.execute(
                "INSERT INTO pr_files (pr_id, position, filename, status, additions, deletions, patch)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    pr_id,
                    i64::try_from(position)?,
                    file.filename,
                    file.status,
                    sql_integer(file.additions)?,
                    sql_integer(file.deletions)?,
                    file.patch
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// The cached changed files, or `None` when missing or fetched for an older
    /// `updated_at` than `pull_request` has.
    pub fn files(&self, pull_request: &PullRequest) -> Result<Option<Vec<PullRequestFile>>> {
        let pr_id = sql_integer(pull_request.id)?;

        // Only a set fetched for this exact version counts.
        let is_current = self
            .connection
            .query_row(
                "SELECT 1 FROM pr_file_sets WHERE pr_id = ?1 AND updated_at = ?2",
                params![pr_id, pull_request.updated_at],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !is_current {
            return Ok(None);
        }

        let mut statement = self.connection.prepare(
            "SELECT filename, status, additions, deletions, patch
             FROM pr_files WHERE pr_id = ?1 ORDER BY position",
        )?;
        let rows = statement.query_map([pr_id], pull_request_file_from_row)?;
        Ok(Some(rows.collect::<rusqlite::Result<_>>()?))
    }
}

/// Reads a pull request from a row of the `list` query's columns, in order.
fn pull_request_from_row(row: &Row<'_>) -> rusqlite::Result<PullRequest> {
    Ok(PullRequest {
        id: read_u64(row, 0)?,
        repo: RepoRef {
            owner: row.get(1)?,
            name: row.get(2)?,
        },
        number: read_u64(row, 3)?,
        title: row.get(4)?,
        body: row.get(5)?,
        author: User {
            login: row.get(6)?,
            avatar_url: row.get(7)?,
        },
        html_url: row.get(8)?,
        draft: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

/// Reads a changed file from a row of the `files` query's columns, in order.
fn pull_request_file_from_row(row: &Row<'_>) -> rusqlite::Result<PullRequestFile> {
    Ok(PullRequestFile {
        filename: row.get(0)?,
        status: row.get(1)?,
        additions: read_u64(row, 2)?,
        deletions: read_u64(row, 3)?,
        patch: row.get(4)?,
    })
}

/// Reads a non-negative integer column; a negative value is a damaged row and
/// an error, never a wrapped number.
fn read_u64(row: &Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
}

/// Converts a GitHub id, number or count to SQLite's signed integer, failing
/// instead of wrapping when it does not fit.
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

    /// Every field of a pull request survives a list replacement and read.
    #[test]
    fn pull_request_fields_round_trip() {
        let mut cache = GitHubCache::open_in_memory().unwrap();
        let mut stored = pull_request(7, "t");
        stored.body = Some("Description".into());
        stored.draft = true;
        stored.html_url = "https://github.com/a/b/pull/7".into();
        cache
            .replace_list(LIST, std::slice::from_ref(&stored))
            .unwrap();
        assert_eq!(cache.list(LIST).unwrap(), [stored]);
    }

    /// Stored files come back in GitHub's order, and a new set replaces the old one.
    #[test]
    fn put_files_keeps_order_and_replaces_previous_set() {
        // Set up: one cached pull request.
        let mut cache = GitHubCache::open_in_memory().unwrap();
        let cached = pull_request(1, "t");
        cache
            .replace_list(LIST, std::slice::from_ref(&cached))
            .unwrap();
        let file = |filename: &str, patch: Option<&str>| PullRequestFile {
            filename: filename.into(),
            status: "modified".into(),
            additions: 3,
            deletions: 1,
            patch: patch.map(Into::into),
        };

        // Act: store two files, then a different single file.
        cache
            .put_files(
                &cached,
                &[file("b.rs", Some("@@ -1 +1 @@")), file("a.rs", None)],
            )
            .unwrap();
        let first = cache.files(&cached).unwrap().unwrap();
        cache.put_files(&cached, &[file("c.rs", None)]).unwrap();

        // Expect: the first read kept order and patches; the second set replaced it.
        assert_eq!(
            first,
            [file("b.rs", Some("@@ -1 +1 @@")), file("a.rs", None)]
        );
        assert_eq!(cache.files(&cached).unwrap().unwrap(), [file("c.rs", None)]);
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
