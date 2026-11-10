use anyhow::Result;
use qd_github::{ETag, PullRequestStack, RepoRef, StackLayer};
use rusqlite::{OptionalExtension, Row, params};

use crate::cached_statements::execute_cached;
use crate::sql_integers::{read_u64, sql_integer};
use crate::unix_now::unix_now;
use crate::{CachedStack, GitHubCache};

/// The columns of a `pr_stacks` row, read before the layers.
struct StoredStack {
    base_ref: String,
    is_open: bool,
    etag: Option<String>,
}

impl GitHubCache {
    /// The cached stack `number` of `repo` with its layers, bottom first,
    /// and marks it used now so cleanup keeps it. `None` when not cached.
    pub fn stack(&self, repo: &RepoRef, number: u64) -> Result<Option<CachedStack>> {
        let number_value = sql_integer(number)?;
        let stored = self
            .connection()
            .query_row(
                "SELECT base_ref, is_open, etag FROM pr_stacks
                 WHERE repo_owner = ?1 AND repo_name = ?2 AND number = ?3",
                params![repo.owner, repo.name, number_value],
                |row| {
                    Ok(StoredStack {
                        base_ref: row.get(0)?,
                        is_open: row.get(1)?,
                        etag: row.get(2)?,
                    })
                },
            )
            .optional()?;
        let Some(stored) = stored else {
            return Ok(None);
        };
        self.touch_stack(repo, number)?;

        let mut statement = self.connection().prepare(
            "SELECT pr_id, pr_number, title, is_open, is_draft, is_merged, html_url,
                    head_ref, head_sha, base_ref, base_sha, author_login
             FROM pr_stack_layers
             WHERE repo_owner = ?1 AND repo_name = ?2 AND stack_number = ?3
             ORDER BY position",
        )?;
        let layers = statement
            .query_map(
                params![repo.owner, repo.name, number_value],
                stack_layer_from_row,
            )?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Some(CachedStack {
            stack: PullRequestStack {
                repo: repo.clone(),
                number,
                base_ref: stored.base_ref,
                is_open: stored.is_open,
                layers,
            },
            etag: stored.etag.map(ETag::new),
        }))
    }

    /// Stores a stack as fetched with the response's tag, replacing the
    /// layers stored before.
    pub fn put_stack(&mut self, stack: &PullRequestStack, etag: Option<&ETag>) -> Result<()> {
        let number = sql_integer(stack.number)?;
        let transaction = self.connection_mut().transaction()?;

        // Record the stack, and clear its old layers.
        transaction.execute(
            "INSERT INTO pr_stacks (repo_owner, repo_name, number, base_ref, is_open, etag, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (repo_owner, repo_name, number) DO UPDATE SET
                base_ref = excluded.base_ref, is_open = excluded.is_open,
                etag = excluded.etag, last_used_at = excluded.last_used_at",
            params![
                stack.repo.owner,
                stack.repo.name,
                number,
                stack.base_ref,
                stack.is_open,
                etag.map(ETag::as_str),
                unix_now()
            ],
        )?;
        transaction.execute(
            "DELETE FROM pr_stack_layers WHERE repo_owner = ?1 AND repo_name = ?2 AND stack_number = ?3",
            params![stack.repo.owner, stack.repo.name, number],
        )?;

        // Insert the layers bottom first.
        for (position, layer) in stack.layers.iter().enumerate() {
            execute_cached(
                &transaction,
                "INSERT INTO pr_stack_layers (
                    repo_owner, repo_name, stack_number, position, pr_id, pr_number, title,
                    is_open, is_draft, is_merged, html_url, head_ref, head_sha, base_ref,
                    base_sha, author_login)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    stack.repo.owner,
                    stack.repo.name,
                    number,
                    i64::try_from(position)?,
                    sql_integer(layer.pull_request_id)?,
                    sql_integer(layer.number)?,
                    layer.title,
                    layer.is_open,
                    layer.is_draft,
                    layer.is_merged,
                    layer.html_url,
                    layer.head_ref,
                    layer.head_sha,
                    layer.base_ref,
                    layer.base_sha,
                    layer.author_login
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Marks a cached stack used now: GitHub confirmed it unchanged.
    pub fn touch_stack(&self, repo: &RepoRef, number: u64) -> Result<()> {
        self.connection().execute(
            "UPDATE pr_stacks SET last_used_at = ?4
             WHERE repo_owner = ?1 AND repo_name = ?2 AND number = ?3",
            params![repo.owner, repo.name, sql_integer(number)?, unix_now()],
        )?;
        Ok(())
    }
}

/// Reads a layer from a row of the `stack` query's layer columns, in order.
fn stack_layer_from_row(row: &Row<'_>) -> rusqlite::Result<StackLayer> {
    Ok(StackLayer {
        pull_request_id: read_u64(row, 0)?,
        number: read_u64(row, 1)?,
        title: row.get(2)?,
        is_open: row.get(3)?,
        is_draft: row.get(4)?,
        is_merged: row.get(5)?,
        html_url: row.get(6)?,
        head_ref: row.get(7)?,
        head_sha: row.get(8)?,
        base_ref: row.get(9)?,
        base_sha: row.get(10)?,
        author_login: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(number: u64, base_ref: &str) -> StackLayer {
        StackLayer {
            pull_request_id: number * 10,
            number,
            title: format!("Layer {number}"),
            is_open: true,
            is_draft: number == 2,
            is_merged: false,
            html_url: String::new(),
            head_ref: format!("branch-{number}"),
            head_sha: format!("h{number}"),
            base_ref: base_ref.into(),
            base_sha: "b".into(),
            author_login: "octo".into(),
        }
    }

    /// A stored stack reads back with its layers in order; a new version replaces them.
    #[test]
    fn stack_round_trips_and_replaces_layers() {
        let mut cache = GitHubCache::open_in_memory().unwrap();
        let repo = RepoRef {
            owner: "a".into(),
            name: "b".into(),
        };
        assert_eq!(cache.stack(&repo, 9).unwrap(), None);

        let mut stack = PullRequestStack {
            repo: repo.clone(),
            number: 9,
            base_ref: "main".into(),
            is_open: true,
            layers: vec![layer(1, "main"), layer(2, "branch-1")],
        };
        cache.put_stack(&stack, Some(&ETag::new("one"))).unwrap();
        stack.layers.truncate(1);
        cache.put_stack(&stack, Some(&ETag::new("two"))).unwrap();

        let cached = cache.stack(&repo, 9).unwrap().unwrap();
        assert_eq!(cached.stack, stack);
        assert_eq!(cached.etag, Some(ETag::new("two")));
    }
}
