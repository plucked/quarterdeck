//! Local SQLite cache for GitHub data.
//!
//! Entities are stored as JSON blobs next to a few indexed columns, so model
//! changes rarely need a migration. Bump `MIGRATIONS` for schema changes.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result};
use qc_github::{PullRequest, PullRequestFile};
use rusqlite::{Connection, OptionalExtension, params};

/// Append-only. Index + 1 is the resulting `PRAGMA user_version`.
const MIGRATIONS: &[&str] = &[r#"
    CREATE TABLE pull_requests (
        id          INTEGER PRIMARY KEY,
        repo        TEXT    NOT NULL,
        number      INTEGER NOT NULL,
        updated_at  TEXT    NOT NULL,
        fetched_at  INTEGER NOT NULL,
        data        TEXT    NOT NULL
    );
    CREATE UNIQUE INDEX pull_requests_repo_number ON pull_requests (repo, number);

    -- Ordered membership of PRs in named lists (e.g. "review_requested").
    CREATE TABLE pr_lists (
        list_key  TEXT    NOT NULL,
        pr_id     INTEGER NOT NULL REFERENCES pull_requests (id) ON DELETE CASCADE,
        position  INTEGER NOT NULL,
        PRIMARY KEY (list_key, pr_id)
    );

    -- Changed files, valid only for the PR's `updated_at` they were fetched at.
    CREATE TABLE pr_files (
        pr_id       INTEGER PRIMARY KEY REFERENCES pull_requests (id) ON DELETE CASCADE,
        updated_at  TEXT    NOT NULL,
        data        TEXT    NOT NULL
    );
"#];

pub const LIST_REVIEW_REQUESTED: &str = "review_requested";

pub struct Cache {
    conn: Connection,
}

impl Cache {
    /// `~/Library/Application Support/quick-check/cache.sqlite` on macOS.
    pub fn default_path() -> Result<PathBuf> {
        let dir = dirs::data_dir()
            .context("no data directory")?
            .join("quick-check");
        Ok(dir.join("cache.sqlite"))
    }

    pub fn open_default() -> Result<Self> {
        Self::open(&Self::default_path()?)
    }

    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        let mut cache = Self { conn };
        cache.migrate()?;
        Ok(cache)
    }

    fn migrate(&mut self) -> Result<()> {
        let current: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (ix, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)
                .with_context(|| format!("migration {}", ix + 1))?;
            tx.pragma_update(None, "user_version", ix as i64 + 1)?;
            tx.commit()?;
        }
        Ok(())
    }

    /// Replace the contents of a named PR list, upserting the PRs themselves.
    pub fn replace_list(&mut self, list_key: &str, prs: &[PullRequest]) -> Result<()> {
        let now = unix_now();
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM pr_lists WHERE list_key = ?1", [list_key])?;
        for (position, pr) in prs.iter().enumerate() {
            tx.execute(
                "INSERT INTO pull_requests (id, repo, number, updated_at, fetched_at, data)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (id) DO UPDATE SET
                    updated_at = excluded.updated_at,
                    fetched_at = excluded.fetched_at,
                    data       = excluded.data",
                params![
                    pr.id as i64,
                    pr.repo.full_name(),
                    pr.number as i64,
                    pr.updated_at,
                    now,
                    serde_json::to_string(pr)?
                ],
            )?;
            tx.execute(
                "INSERT INTO pr_lists (list_key, pr_id, position) VALUES (?1, ?2, ?3)",
                params![list_key, pr.id as i64, position as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// PRs of a named list in stored order.
    pub fn list(&self, list_key: &str) -> Result<Vec<PullRequest>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.data FROM pr_lists l
             JOIN pull_requests p ON p.id = l.pr_id
             WHERE l.list_key = ?1 ORDER BY l.position",
        )?;
        let rows = stmt.query_map([list_key], |r| r.get::<_, String>(0))?;
        rows.map(|data| Ok(serde_json::from_str(&data?)?)).collect()
    }

    pub fn put_files(&self, pr: &PullRequest, files: &[PullRequestFile]) -> Result<()> {
        self.conn.execute(
            "INSERT INTO pr_files (pr_id, updated_at, data) VALUES (?1, ?2, ?3)
             ON CONFLICT (pr_id) DO UPDATE SET updated_at = excluded.updated_at, data = excluded.data",
            params![pr.id as i64, pr.updated_at, serde_json::to_string(files)?],
        )?;
        Ok(())
    }

    /// Cached files, or `None` if missing or stale relative to `pr.updated_at`.
    pub fn files(&self, pr: &PullRequest) -> Result<Option<Vec<PullRequestFile>>> {
        let data: Option<String> = self
            .conn
            .query_row(
                "SELECT data FROM pr_files WHERE pr_id = ?1 AND updated_at = ?2",
                params![pr.id as i64, pr.updated_at],
                |r| r.get(0),
            )
            .optional()?;
        data.map(|d| Ok(serde_json::from_str(&d)?)).transpose()
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qc_github::{RepoRef, User};

    fn pr(id: u64, updated_at: &str) -> PullRequest {
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

    #[test]
    fn migrations_are_idempotent() {
        let dir = std::env::temp_dir().join(format!("qc-cache-test-{}", std::process::id()));
        let path = dir.join("cache.sqlite");
        Cache::open(&path).unwrap();
        Cache::open(&path).unwrap();
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn replace_list_keeps_order_and_drops_old_members() {
        let mut cache = Cache::open_in_memory().unwrap();
        cache.replace_list("k", &[pr(1, "t"), pr(2, "t")]).unwrap();
        cache.replace_list("k", &[pr(3, "t"), pr(1, "t2")]).unwrap();
        let ids: Vec<_> = cache.list("k").unwrap().iter().map(|p| p.id).collect();
        assert_eq!(ids, [3, 1]);
        assert_eq!(cache.list("k").unwrap()[1].updated_at, "t2");
    }

    #[test]
    fn files_invalidate_on_pr_update() {
        let mut cache = Cache::open_in_memory().unwrap();
        let old = pr(1, "t1");
        cache.replace_list("k", std::slice::from_ref(&old)).unwrap();
        let file = PullRequestFile {
            filename: "a.rs".into(),
            status: "modified".into(),
            additions: 1,
            deletions: 0,
            patch: None,
        };
        cache.put_files(&old, &[file]).unwrap();
        assert_eq!(cache.files(&old).unwrap().unwrap().len(), 1);
        assert!(cache.files(&pr(1, "t2")).unwrap().is_none());
    }
}
