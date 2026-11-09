use std::time::Duration;

use anyhow::Result;

use crate::unix_now::unix_now;
use crate::{CleanupCounts, GitHubCache};

impl GitHubCache {
    /// Removes cached data not used for longer than `ttl`: changed files and
    /// combined diffs not read or refetched since, stacks not read or
    /// confirmed since, file contents not read since, and pull requests no
    /// longer in any list and not fetched since. Pull requests still in a list (the queue) stay.
    pub fn delete_expired(&mut self, ttl: Duration) -> Result<CleanupCounts> {
        let ttl_seconds = i64::try_from(ttl.as_secs()).unwrap_or(i64::MAX);
        let cutoff = unix_now().saturating_sub(ttl_seconds);
        let transaction = self.connection_mut().transaction()?;

        // Deleting a set deletes its files (ON DELETE CASCADE).
        let file_sets =
            transaction.execute("DELETE FROM pr_file_sets WHERE last_used_at < ?1", [cutoff])?;
        let comparisons = transaction.execute(
            "DELETE FROM compare_file_sets WHERE last_used_at < ?1",
            [cutoff],
        )?;
        let stacks =
            transaction.execute("DELETE FROM pr_stacks WHERE last_used_at < ?1", [cutoff])?;
        let file_texts =
            transaction.execute("DELETE FROM file_texts WHERE last_used_at < ?1", [cutoff])?;
        let pull_requests = transaction.execute(
            "DELETE FROM pull_requests
             WHERE fetched_at < ?1 AND id NOT IN (SELECT pr_id FROM pr_lists)",
            [cutoff],
        )?;
        transaction.commit()?;
        Ok(CleanupCounts {
            file_sets,
            pull_requests,
            comparisons,
            stacks,
            file_texts,
        })
    }

    /// Removes every cached pull request, list and file, and shrinks the
    /// database file to what is left; the next refresh fetches them again.
    /// User state (viewed marks, settings) stays.
    pub fn clear(&mut self) -> Result<()> {
        // Lists and file sets go with their pull requests, layers with their stacks and files with their comparisons (ON DELETE CASCADE).
        self.connection().execute("DELETE FROM pull_requests", [])?;
        self.connection().execute("DELETE FROM pr_stacks", [])?;
        self.connection()
            .execute("DELETE FROM compare_file_sets", [])?;
        self.connection().execute("DELETE FROM file_texts", [])?;

        // SQLite keeps the freed pages in the file; give them back to the disk.
        self.connection()
            .execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use qd_github::{ETag, PullRequest, PullRequestFile, PullRequestRevision, RepoRef, User};
    use rusqlite::params;

    use super::*;
    use crate::{FileSetRevision, PullRequestListKey};

    const LIST: PullRequestListKey = PullRequestListKey::REVIEW_REQUESTED;
    const ONE_DAY: Duration = Duration::from_hours(24);

    fn pull_request(id: u64) -> PullRequest {
        PullRequest {
            id,
            number: id,
            repo: RepoRef {
                owner: "a".into(),
                name: "b".into(),
            },
            title: String::new(),
            body: None,
            author: User {
                login: String::new(),
                avatar_url: String::new(),
            },
            html_url: String::new(),
            draft: false,
            created_at: String::new(),
            updated_at: "t".into(),
            branches: None,
            viewer_review: None,
            labels: Vec::new(),
            stack: None,
        }
    }

    fn revision() -> FileSetRevision {
        FileSetRevision {
            commits: PullRequestRevision {
                head_sha: "head".into(),
                base_sha: "base".into(),
            },
            etag: Some(ETag::new("tag")),
        }
    }

    fn file() -> PullRequestFile {
        PullRequestFile {
            filename: "a.rs".into(),
            previous_filename: None,
            status: "added".into(),
            additions: 1,
            deletions: 0,
            patch: None,
            sha: None,
        }
    }

    /// Ages every cached row by `seconds`, as if that much time had passed.
    fn age_everything(cache: &GitHubCache, seconds: i64) {
        cache
            .connection()
            .execute(
                "UPDATE pr_file_sets SET last_used_at = last_used_at - ?1",
                params![seconds],
            )
            .unwrap();
        cache
            .connection()
            .execute(
                "UPDATE pull_requests SET fetched_at = fetched_at - ?1",
                params![seconds],
            )
            .unwrap();
    }

    /// Old files go; pull requests stay while they are in the queue and go once they left it.
    #[test]
    fn deletes_expired_files_and_unlisted_pull_requests() {
        // Set up: two pull requests with files, then the second leaves the queue.
        let mut cache = GitHubCache::open_in_memory().unwrap();
        cache
            .replace_list(LIST, &[pull_request(1), pull_request(2)])
            .unwrap();
        cache
            .put_files(&pull_request(1), &[file()], &revision())
            .unwrap();
        cache
            .put_files(&pull_request(2), &[file()], &revision())
            .unwrap();
        cache.replace_list(LIST, &[pull_request(1)]).unwrap();

        // Act: two days pass with a one-day TTL.
        age_everything(&cache, 2 * 24 * 60 * 60);
        let counts = cache.delete_expired(ONE_DAY).unwrap();

        // Expect: both file sets expired; only the unlisted pull request went.
        assert_eq!(
            counts,
            CleanupCounts {
                file_sets: 2,
                pull_requests: 1,
                ..CleanupCounts::default()
            }
        );
        assert_eq!(cache.list(LIST).unwrap().len(), 1);
        assert_eq!(cache.file_set(1).unwrap(), None);
    }

    /// Data used within the TTL stays.
    #[test]
    fn keeps_recent_data() {
        let mut cache = GitHubCache::open_in_memory().unwrap();
        cache.replace_list(LIST, &[pull_request(1)]).unwrap();
        cache
            .put_files(&pull_request(1), &[file()], &revision())
            .unwrap();
        assert_eq!(
            cache.delete_expired(ONE_DAY).unwrap(),
            CleanupCounts::default()
        );
        assert!(cache.file_set(1).unwrap().is_some());
    }

    /// Clearing removes lists, pull requests and files.
    #[test]
    fn clear_removes_everything_cached() {
        let mut cache = GitHubCache::open_in_memory().unwrap();
        cache.replace_list(LIST, &[pull_request(1)]).unwrap();
        cache
            .put_files(&pull_request(1), &[file()], &revision())
            .unwrap();
        cache.clear().unwrap();
        assert_eq!(cache.list(LIST).unwrap().len(), 0);
        assert_eq!(cache.file_set(1).unwrap(), None);
    }
}
