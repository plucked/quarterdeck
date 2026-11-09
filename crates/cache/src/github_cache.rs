use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use qd_github::{Label, PullRequest, PullRequestBranches, RepoRef, StackMembership, User};
use rusqlite::{Connection, Row, Transaction, params};

use crate::PullRequestListKey;
use crate::database_connection::{default_database_path, open_database, prepare_data_directory};
use crate::sql_integers::{read_u64, sql_integer};
use crate::unix_now::unix_now;

/// The columns [`pull_request_from_row`] reads, in order, from `pull_requests p`.
const PULL_REQUEST_COLUMNS: &str = "p.id, p.repo_owner, p.repo_name, p.number, p.title, p.body,
    p.author_login, p.author_avatar_url, p.html_url, p.draft, p.created_at, p.updated_at,
    p.stack_number, p.stack_position, p.stack_size, p.stack_base_ref, p.head_ref, p.base_ref";

/// The SQLite connection holding cached GitHub data, one typed column per
/// field: pull requests and lists here, changed files in
/// `file_set_storage.rs`, expiry in `cache_cleanup.rs`. Every write runs in one statement or one transaction, so the
/// database is consistent after any failure.
pub struct GitHubCache {
    connection: Connection,
}

impl GitHubCache {
    /// Opens (creating if needed) the default database file.
    pub fn open_default() -> Result<Self> {
        prepare_data_directory()?;
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
            let id = upsert_pull_request(&transaction, pull_request, fetched_at)?;
            transaction.execute(
                "INSERT INTO pr_lists (list_key, pr_id, position) VALUES (?1, ?2, ?3)",
                params![list_key.as_str(), id, i64::try_from(position)?],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Stores one pull request outside any list (a stack layer opened from
    /// the stack map), so a relaunch can show it again.
    pub fn put_pull_request(&mut self, pull_request: &PullRequest) -> Result<()> {
        let transaction = self.connection.transaction()?;
        upsert_pull_request(&transaction, pull_request, unix_now())?;
        transaction.commit()?;
        Ok(())
    }

    /// The cached pull request with GitHub id `id`, in a list or not.
    pub fn pull_request(&self, id: u64) -> Result<Option<PullRequest>> {
        let mut statement = self.connection.prepare(&format!(
            "SELECT {PULL_REQUEST_COLUMNS} FROM pull_requests p WHERE p.id = ?1"
        ))?;
        let mut rows = statement.query_map([sql_integer(id)?], pull_request_from_row)?;
        let Some(mut pull_request) = rows.next().transpose()? else {
            return Ok(None);
        };
        pull_request.labels = self.labels_of(pull_request.id)?;
        Ok(Some(pull_request))
    }

    /// The cached pull request `number` of `repo`, in a list or not (search
    /// and pulls ids differ, numbers agree).
    pub fn pull_request_by_number(
        &self,
        repo: &RepoRef,
        number: u64,
    ) -> Result<Option<PullRequest>> {
        let mut statement = self.connection.prepare_cached(&format!(
            "SELECT {PULL_REQUEST_COLUMNS} FROM pull_requests p
             WHERE p.repo_owner = ?1 AND p.repo_name = ?2 AND p.number = ?3"
        ))?;
        let mut rows = statement.query_map(
            params![repo.owner, repo.name, sql_integer(number)?],
            pull_request_from_row,
        )?;
        let Some(mut pull_request) = rows.next().transpose()? else {
            return Ok(None);
        };
        pull_request.labels = self.labels_of(pull_request.id)?;
        Ok(Some(pull_request))
    }

    /// The pull requests of a list, in stored order.
    pub fn list(&self, list_key: PullRequestListKey) -> Result<Vec<PullRequest>> {
        let mut statement = self.connection.prepare(&format!(
            "SELECT {PULL_REQUEST_COLUMNS}
             FROM pr_lists l
             JOIN pull_requests p ON p.id = l.pr_id
             WHERE l.list_key = ?1 ORDER BY l.position"
        ))?;
        let rows = statement.query_map([list_key.as_str()], pull_request_from_row)?;
        let mut pull_requests = rows.collect::<rusqlite::Result<Vec<_>>>()?;

        // Every member's labels in one query, not one per pull request.
        let mut labels = self.labels_of_list(list_key)?;
        for pull_request in &mut pull_requests {
            pull_request.labels = labels.remove(&pull_request.id).unwrap_or_default();
        }
        Ok(pull_requests)
    }

    /// The stored labels of every pull request in a list, by pull request
    /// id, each in GitHub's order.
    fn labels_of_list(&self, list_key: PullRequestListKey) -> Result<HashMap<u64, Vec<Label>>> {
        let mut statement = self.connection.prepare_cached(
            "SELECT b.pr_id, b.name, b.color FROM pr_labels b
             JOIN pr_lists l ON l.pr_id = b.pr_id
             WHERE l.list_key = ?1 ORDER BY b.pr_id, b.position",
        )?;
        let rows = statement.query_map([list_key.as_str()], |row| {
            Ok((
                read_u64(row, 0)?,
                Label {
                    name: row.get(1)?,
                    color: row.get(2)?,
                },
            ))
        })?;
        let mut labels: HashMap<u64, Vec<Label>> = HashMap::new();
        for row in rows {
            let (id, label) = row?;
            labels.entry(id).or_default().push(label);
        }
        Ok(labels)
    }

    /// The stored labels of pull request `id`, in GitHub's order.
    fn labels_of(&self, id: u64) -> Result<Vec<Label>> {
        let mut statement = self.connection.prepare_cached(
            "SELECT name, color FROM pr_labels WHERE pr_id = ?1 ORDER BY position",
        )?;
        let labels = statement
            .query_map([sql_integer(id)?], |row| {
                Ok(Label {
                    name: row.get(0)?,
                    color: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(labels)
    }

    /// The connection, for the storage of other cached topics in sibling files.
    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }

    /// The connection for writes that need a transaction.
    pub(crate) fn connection_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }
}

/// Inserts or updates one pull request row; returns its id as stored.
fn upsert_pull_request(
    transaction: &Transaction<'_>,
    pull_request: &PullRequest,
    fetched_at: i64,
) -> Result<i64> {
    let id = sql_integer(pull_request.id)?;
    upsert_pull_request_row(transaction, pull_request, id, fetched_at)?;

    // Its labels replace the ones stored before.
    transaction.execute("DELETE FROM pr_labels WHERE pr_id = ?1", [id])?;
    for (position, label) in pull_request.labels.iter().enumerate() {
        transaction.execute(
            "INSERT INTO pr_labels (pr_id, position, name, color) VALUES (?1, ?2, ?3, ?4)",
            params![id, i64::try_from(position)?, label.name, label.color],
        )?;
    }
    Ok(id)
}

/// Inserts or updates the `pull_requests` row of `pull_request`.
fn upsert_pull_request_row(
    transaction: &Transaction<'_>,
    pull_request: &PullRequest,
    id: i64,
    fetched_at: i64,
) -> Result<()> {
    let stack = pull_request.stack.as_ref();
    let branches = pull_request.branches.as_ref();
    transaction.execute(
        "INSERT INTO pull_requests (
            id, repo_owner, repo_name, number, title, body, author_login,
            author_avatar_url, html_url, draft, created_at, updated_at, fetched_at,
            stack_number, stack_position, stack_size, stack_base_ref, head_ref, base_ref)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
            ?18, ?19)
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
            fetched_at        = excluded.fetched_at,
            stack_number      = excluded.stack_number,
            stack_position    = excluded.stack_position,
            stack_size        = excluded.stack_size,
            stack_base_ref    = excluded.stack_base_ref,
            head_ref          = excluded.head_ref,
            base_ref          = excluded.base_ref",
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
            fetched_at,
            stack.map(|stack| sql_integer(stack.number)).transpose()?,
            stack.map(|stack| sql_integer(stack.position)).transpose()?,
            stack.map(|stack| sql_integer(stack.size)).transpose()?,
            stack.map(|stack| stack.base_ref.as_str()),
            branches.map(|branches| branches.head_ref.as_str()),
            branches.map(|branches| branches.base_ref.as_str()),
        ],
    )?;
    Ok(())
}

/// Reads a pull request from a row of [`PULL_REQUEST_COLUMNS`].
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
        // Read from `pr_labels` by the caller.
        labels: Vec::new(),
        stack: stack_membership_from_row(row)?,
        branches: branches_from_row(row)?,
    })
}

/// The branch columns of a pull request row; `None` unless both are set.
fn branches_from_row(row: &Row<'_>) -> rusqlite::Result<Option<PullRequestBranches>> {
    let head_ref: Option<String> = row.get(16)?;
    let base_ref: Option<String> = row.get(17)?;
    Ok(head_ref
        .zip(base_ref)
        .map(|(head_ref, base_ref)| PullRequestBranches { head_ref, base_ref }))
}

/// The stack columns of a pull request row; `None` unless all are set.
fn stack_membership_from_row(row: &Row<'_>) -> rusqlite::Result<Option<StackMembership>> {
    let number: Option<i64> = row.get(12)?;
    let base_ref: Option<String> = row.get(15)?;
    let (Some(_), Some(base_ref)) = (number, base_ref) else {
        return Ok(None);
    };
    Ok(Some(StackMembership {
        number: read_u64(row, 12)?,
        position: read_u64(row, 13)?,
        size: read_u64(row, 14)?,
        base_ref,
    }))
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
            branches: None,
            labels: Vec::new(),
            stack: None,
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

    /// A list's pull requests come back with their own labels, in order.
    #[test]
    fn list_reads_each_pull_requests_labels() {
        let mut cache = GitHubCache::open_in_memory().unwrap();
        let label = |name: &str| Label {
            name: name.into(),
            color: "ffffff".into(),
        };
        let mut first = pull_request(1, "t");
        first.labels = vec![label("bug"), label("ui")];
        let second = pull_request(2, "t");
        cache.replace_list(LIST, &[first, second]).unwrap();

        let listed = cache.list(LIST).unwrap();
        assert_eq!(listed[0].labels, [label("bug"), label("ui")]);
        assert_eq!(listed[1].labels, Vec::<Label>::new());
    }

    /// Every field of a pull request survives a list replacement and read.
    #[test]
    fn pull_request_fields_round_trip() {
        let mut cache = GitHubCache::open_in_memory().unwrap();
        let mut stored = pull_request(7, "t");
        stored.body = Some("Description".into());
        stored.draft = true;
        stored.html_url = "https://github.com/a/b/pull/7".into();
        stored.stack = Some(StackMembership {
            number: 3,
            position: 2,
            size: 4,
            base_ref: "main".into(),
        });
        stored.branches = Some(PullRequestBranches {
            head_ref: "feat/x".into(),
            base_ref: "main".into(),
        });
        cache
            .replace_list(LIST, std::slice::from_ref(&stored))
            .unwrap();
        assert_eq!(cache.list(LIST).unwrap(), [stored]);
    }
}
