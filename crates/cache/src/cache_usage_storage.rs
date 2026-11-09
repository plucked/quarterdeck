use std::path::PathBuf;

use anyhow::Result;
use rusqlite::Connection;

use crate::{CacheUsage, CachedData, GitHubCache};

/// Bytes of a set of text columns: `LENGTH` of a `BLOB` counts bytes, of
/// `TEXT` characters.
const PULL_REQUEST_BYTES: &str = "SELECT COUNT(*), 0, COALESCE(SUM(
        LENGTH(CAST(title AS BLOB)) + COALESCE(LENGTH(CAST(body AS BLOB)), 0)
        + LENGTH(CAST(html_url AS BLOB)) + LENGTH(CAST(author_avatar_url AS BLOB))), 0)
    FROM pull_requests";
const FILE_SET_BYTES: &str = "SELECT
        (SELECT COUNT(*) FROM pr_file_sets), COUNT(*),
        COALESCE(SUM(LENGTH(CAST(filename AS BLOB)) + COALESCE(LENGTH(CAST(patch AS BLOB)), 0)), 0)
    FROM pr_files";
const COMPARISON_BYTES: &str = "SELECT
        (SELECT COUNT(*) FROM compare_file_sets), COUNT(*),
        COALESCE(SUM(LENGTH(CAST(filename AS BLOB)) + COALESCE(LENGTH(CAST(patch AS BLOB)), 0)), 0)
    FROM compare_files";
const FILE_TEXT_BYTES: &str =
    "SELECT COUNT(*), COUNT(*), COALESCE(SUM(LENGTH(CAST(text AS BLOB))), 0) FROM file_texts";

impl GitHubCache {
    /// How much each kind of cached data takes, and the database on disk.
    pub fn usage(&self) -> Result<CacheUsage> {
        let connection = self.connection();
        let page_size = pragma_number(connection, "page_size")?;
        let pages = pragma_number(connection, "page_count")?;
        let free_pages = pragma_number(connection, "freelist_count")?;
        Ok(CacheUsage {
            pull_requests: cached_data(connection, PULL_REQUEST_BYTES)?,
            pull_request_files: cached_data(connection, FILE_SET_BYTES)?,
            comparisons: cached_data(connection, COMPARISON_BYTES)?,
            file_texts: cached_data(connection, FILE_TEXT_BYTES)?,
            disk_bytes: page_size * pages + write_ahead_log_bytes(connection)?,
            free_bytes: page_size * free_pages,
        })
    }
}

/// Entries, files and bytes from one of the queries above.
fn cached_data(connection: &Connection, query: &str) -> Result<CachedData> {
    let (entries, files, bytes): (i64, i64, i64) =
        connection.query_row(query, [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
    Ok(CachedData {
        entries: u64::try_from(entries)?,
        files: u64::try_from(files)?,
        bytes: u64::try_from(bytes)?,
    })
}

/// A numeric `PRAGMA`'s value.
fn pragma_number(connection: &Connection, pragma: &str) -> Result<u64> {
    let value: i64 = connection.query_row(&format!("PRAGMA {pragma}"), [], |row| row.get(0))?;
    Ok(u64::try_from(value)?)
}

/// The size of the main database's write-ahead log; 0 for an in-memory
/// database or when there is none.
fn write_ahead_log_bytes(connection: &Connection) -> Result<u64> {
    let path: String =
        connection.query_row("PRAGMA database_list", [], |row| row.get::<_, String>(2))?;
    if path.is_empty() {
        return Ok(0);
    }
    let mut log = PathBuf::from(path).into_os_string();
    log.push("-wal");
    Ok(std::fs::metadata(log).map_or(0, |metadata| metadata.len()))
}

#[cfg(test)]
mod tests {
    use qd_github::{PullRequest, RepoRef, User};

    use crate::{GitHubCache, PullRequestListKey};

    fn pull_request(id: u64, title: &str) -> PullRequest {
        PullRequest {
            id,
            number: id,
            repo: RepoRef {
                owner: "a".into(),
                name: "b".into(),
            },
            title: title.into(),
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
            labels: Vec::new(),
            stack: None,
        }
    }

    /// Cached pull requests count with the bytes of their text; an empty cache counts nothing.
    #[test]
    fn counts_cached_pull_requests() {
        let mut cache = GitHubCache::open_in_memory().unwrap();
        assert_eq!(cache.usage().unwrap().pull_requests.entries, 0);

        cache
            .replace_list(
                PullRequestListKey::AUTHORED,
                &[pull_request(1, "abc"), pull_request(2, "héllo")],
            )
            .unwrap();
        let usage = cache.usage().unwrap();
        assert_eq!(usage.pull_requests.entries, 2);
        assert_eq!(usage.pull_requests.bytes, 3 + 6);
        assert!(usage.disk_bytes > 0);
    }
}
