use anyhow::Result;
use qd_github::{ETag, PullRequest, PullRequestFile, PullRequestRevision};
use rusqlite::{OptionalExtension, Row, params};

use crate::cached_statements::execute_cached;
use crate::sql_integers::{read_u64, sql_integer};
use crate::unix_now::unix_now;
use crate::{CachedFileSet, FileSetRevision, GitHubCache};

/// The columns of a `pr_file_sets` row, read before the files.
struct StoredFileSet {
    updated_at: String,
    head_sha: String,
    base_sha: String,
    revision_etag: Option<String>,
}

impl GitHubCache {
    /// The cached changed files of a pull request, in whatever version they
    /// were fetched or confirmed for, and marks them used now so cleanup
    /// keeps them. `None` when none are cached.
    pub fn file_set(&self, pull_request_id: u64) -> Result<Option<CachedFileSet>> {
        let pr_id = sql_integer(pull_request_id)?;
        let stored = self
            .connection()
            .query_row(
                "SELECT updated_at, head_sha, base_sha, revision_etag
                 FROM pr_file_sets WHERE pr_id = ?1",
                [pr_id],
                |row| {
                    Ok(StoredFileSet {
                        updated_at: row.get(0)?,
                        head_sha: row.get(1)?,
                        base_sha: row.get(2)?,
                        revision_etag: row.get(3)?,
                    })
                },
            )
            .optional()?;
        let Some(stored) = stored else {
            return Ok(None);
        };

        // Being read is a use: it moves the set's expiry.
        self.connection().execute(
            "UPDATE pr_file_sets SET last_used_at = ?2 WHERE pr_id = ?1",
            params![pr_id, unix_now()],
        )?;

        let mut statement = self.connection().prepare(
            "SELECT filename, previous_filename, status, additions, deletions, patch, sha
             FROM pr_files WHERE pr_id = ?1 ORDER BY position",
        )?;
        let rows = statement.query_map([pr_id], pull_request_file_from_row)?;
        Ok(Some(CachedFileSet {
            updated_at: stored.updated_at,
            revision: FileSetRevision {
                commits: PullRequestRevision {
                    head_sha: stored.head_sha,
                    base_sha: stored.base_sha,
                },
                etag: stored.revision_etag.map(ETag::new),
            },
            files: rows.collect::<rusqlite::Result<_>>()?,
        }))
    }

    /// The commits the cached files of a pull request were fetched for,
    /// without reading the files. `None` when none are cached.
    pub fn file_set_commits(&self, pull_request_id: u64) -> Result<Option<PullRequestRevision>> {
        let commits = self
            .connection()
            .query_row(
                "SELECT head_sha, base_sha FROM pr_file_sets WHERE pr_id = ?1",
                [sql_integer(pull_request_id)?],
                |row| {
                    Ok(PullRequestRevision {
                        head_sha: row.get(0)?,
                        base_sha: row.get(1)?,
                    })
                },
            )
            .optional()?;
        Ok(commits)
    }

    /// Stores the changed files of `pull_request` as fetched for `revision`,
    /// replacing any stored before. The pull request must already be cached.
    pub fn put_files(
        &mut self,
        pull_request: &PullRequest,
        files: &[PullRequestFile],
        revision: &FileSetRevision,
    ) -> Result<()> {
        let pr_id = sql_integer(pull_request.id)?;
        let transaction = self.connection_mut().transaction()?;

        // Record what the set belongs to, and clear the old files.
        transaction.execute(
            "INSERT INTO pr_file_sets (pr_id, updated_at, head_sha, base_sha, revision_etag, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (pr_id) DO UPDATE SET
                updated_at = excluded.updated_at, head_sha = excluded.head_sha,
                base_sha = excluded.base_sha, revision_etag = excluded.revision_etag,
                last_used_at = excluded.last_used_at",
            params![
                pr_id,
                pull_request.updated_at,
                revision.commits.head_sha,
                revision.commits.base_sha,
                revision.etag.as_ref().map(ETag::as_str),
                unix_now()
            ],
        )?;
        transaction.execute("DELETE FROM pr_files WHERE pr_id = ?1", [pr_id])?;

        // Insert the files in GitHub's order.
        for (position, file) in files.iter().enumerate() {
            execute_cached(
                &transaction,
                "INSERT INTO pr_files (
                    pr_id, position, filename, previous_filename, status, additions, deletions,
                    patch, sha)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    pr_id,
                    i64::try_from(position)?,
                    file.filename,
                    file.previous_filename,
                    file.status,
                    sql_integer(file.additions)?,
                    sql_integer(file.deletions)?,
                    file.patch,
                    file.sha
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Records that the cached files are still those of `pull_request` as it
    /// is now: its commits did not change. Keeps the files, moves the set to
    /// the pull request's `updated_at` and the response tag in `revision`.
    pub fn confirm_files(
        &self,
        pull_request: &PullRequest,
        revision: &FileSetRevision,
    ) -> Result<()> {
        self.connection().execute(
            "UPDATE pr_file_sets
             SET updated_at = ?2, revision_etag = ?3, last_used_at = ?4
             WHERE pr_id = ?1",
            params![
                sql_integer(pull_request.id)?,
                pull_request.updated_at,
                revision.etag.as_ref().map(ETag::as_str),
                unix_now()
            ],
        )?;
        Ok(())
    }
}

/// Reads a changed file from a row of the `file_set` query's columns, in
/// order (the comparison query reads the same columns).
pub(crate) fn pull_request_file_from_row(row: &Row<'_>) -> rusqlite::Result<PullRequestFile> {
    Ok(PullRequestFile {
        filename: row.get(0)?,
        previous_filename: row.get(1)?,
        status: row.get(2)?,
        additions: read_u64(row, 3)?,
        deletions: read_u64(row, 4)?,
        patch: row.get(5)?,
        sha: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use qd_github::{RepoRef, User};

    use super::*;
    use crate::PullRequestListKey;

    fn pull_request(updated_at: &str) -> PullRequest {
        PullRequest {
            id: 1,
            number: 1,
            repo: RepoRef {
                owner: "a".into(),
                name: "b".into(),
            },
            title: "PR".into(),
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
            viewer_review: None,
            labels: Vec::new(),
            stack: None,
        }
    }

    fn file(filename: &str) -> PullRequestFile {
        PullRequestFile {
            filename: filename.into(),
            previous_filename: None,
            status: "modified".into(),
            additions: 3,
            deletions: 1,
            patch: Some("@@ -1 +1 @@".into()),
            sha: None,
        }
    }

    fn revision(head: &str, etag: &str) -> FileSetRevision {
        FileSetRevision {
            commits: PullRequestRevision {
                head_sha: head.into(),
                base_sha: "base".into(),
            },
            etag: Some(ETag::new(etag)),
        }
    }

    fn cache_with(pull_request: &PullRequest) -> GitHubCache {
        let mut cache = GitHubCache::open_in_memory().unwrap();
        cache
            .replace_list(
                PullRequestListKey::REVIEW_REQUESTED,
                std::slice::from_ref(pull_request),
            )
            .unwrap();
        cache
    }

    /// A stored set reads back with its files in order and what it was fetched for.
    #[test]
    fn file_set_round_trips() {
        let pull_request = pull_request("t1");
        let mut cache = cache_with(&pull_request);
        cache
            .put_files(
                &pull_request,
                &[file("b.rs"), file("a.rs")],
                &revision("head1", "tag1"),
            )
            .unwrap();

        let set = cache.file_set(1).unwrap().unwrap();
        assert_eq!(
            cache.file_set_commits(1).unwrap(),
            Some(set.revision.commits.clone())
        );
        assert_eq!(set.files, [file("b.rs"), file("a.rs")]);
        assert_eq!(set.revision, revision("head1", "tag1"));
        assert!(set.is_current_for(&pull_request));
        assert!(!set.is_current_for(&self::pull_request("t2")));
    }

    /// Confirming keeps the files and commits but moves the set to the new version and tag.
    #[test]
    fn confirm_moves_set_to_new_version() {
        let old = pull_request("t1");
        let mut cache = cache_with(&old);
        cache
            .put_files(&old, &[file("a.rs")], &revision("head1", "tag1"))
            .unwrap();

        let commented = pull_request("t2");
        cache
            .confirm_files(&commented, &revision("head1", "tag2"))
            .unwrap();

        let set = cache.file_set(1).unwrap().unwrap();
        assert!(set.is_current_for(&commented));
        assert_eq!(set.files, [file("a.rs")]);
        assert_eq!(set.revision.etag, Some(ETag::new("tag2")));
    }

    /// A pull request without cached files has no set.
    #[test]
    fn missing_set_is_none() {
        let cache = cache_with(&pull_request("t1"));
        assert_eq!(cache.file_set(1).unwrap(), None);
    }
}
