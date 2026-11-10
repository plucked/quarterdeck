use anyhow::Result;
use qd_github::{PullRequestFile, RepoRef};
use rusqlite::params;

use crate::GitHubCache;
use crate::cached_statements::execute_cached;
use crate::file_set_storage::pull_request_file_from_row;
use crate::sql_integers::sql_integer;
use crate::unix_now::unix_now;

impl GitHubCache {
    /// The cached changed files from `base_sha` to `head_sha` in `repo`, and
    /// marks them used now. `None` when not cached. Commits never change, so
    /// cached files are always current.
    pub fn compare_files(
        &self,
        repo: &RepoRef,
        base_sha: &str,
        head_sha: &str,
    ) -> Result<Option<Vec<PullRequestFile>>> {
        let key = params![repo.owner, repo.name, base_sha, head_sha];
        let touched = self.connection().execute(
            "UPDATE compare_file_sets SET last_used_at = ?5
             WHERE repo_owner = ?1 AND repo_name = ?2 AND base_sha = ?3 AND head_sha = ?4",
            params![repo.owner, repo.name, base_sha, head_sha, unix_now()],
        )?;
        if touched == 0 {
            return Ok(None);
        }
        let mut statement = self.connection().prepare(
            "SELECT filename, previous_filename, status, additions, deletions, patch, sha
             FROM compare_files
             WHERE repo_owner = ?1 AND repo_name = ?2 AND base_sha = ?3 AND head_sha = ?4
             ORDER BY position",
        )?;
        let files = statement
            .query_map(key, pull_request_file_from_row)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Some(files))
    }

    /// Stores the changed files from `base_sha` to `head_sha`, replacing any
    /// stored before.
    pub fn put_compare_files(
        &mut self,
        repo: &RepoRef,
        base_sha: &str,
        head_sha: &str,
        files: &[PullRequestFile],
    ) -> Result<()> {
        let transaction = self.connection_mut().transaction()?;

        // Record the comparison; replacing it drops its old files (ON DELETE CASCADE).
        transaction.execute(
            "DELETE FROM compare_file_sets
             WHERE repo_owner = ?1 AND repo_name = ?2 AND base_sha = ?3 AND head_sha = ?4",
            params![repo.owner, repo.name, base_sha, head_sha],
        )?;
        transaction.execute(
            "INSERT INTO compare_file_sets (repo_owner, repo_name, base_sha, head_sha, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![repo.owner, repo.name, base_sha, head_sha, unix_now()],
        )?;

        // Insert the files in the diff's order.
        for (position, file) in files.iter().enumerate() {
            execute_cached(
                &transaction,
                "INSERT INTO compare_files (
                    repo_owner, repo_name, base_sha, head_sha, position, filename,
                    previous_filename, status, additions, deletions, patch, sha)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    repo.owner,
                    repo.name,
                    base_sha,
                    head_sha,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_pull_requests::sample_file;

    /// A comparison reads back in order, and only for its own commits.
    #[test]
    fn compare_files_round_trip() {
        let mut cache = GitHubCache::open_in_memory().unwrap();
        let repo = RepoRef {
            owner: "a".into(),
            name: "b".into(),
        };
        assert_eq!(cache.compare_files(&repo, "base", "head").unwrap(), None);

        let files = [sample_file("b.rs"), sample_file("a.rs")];
        cache
            .put_compare_files(&repo, "base", "head", &files)
            .unwrap();
        cache
            .put_compare_files(&repo, "base", "head", &files)
            .unwrap();
        assert_eq!(
            cache.compare_files(&repo, "base", "head").unwrap(),
            Some(files.to_vec())
        );
        assert_eq!(cache.compare_files(&repo, "base", "other").unwrap(), None);
    }
}
