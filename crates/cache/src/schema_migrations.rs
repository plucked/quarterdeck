use anyhow::{Context as _, Result};
use rusqlite::Connection;

/// The schema history, append-only: entry `n` (0-based) moves `PRAGMA
/// user_version` from `n` to `n + 1`. Entities are JSON blobs beside a few
/// indexed columns, so model changes rarely need a new entry.
const SCHEMA_MIGRATIONS: &[&str] = &[r"
    CREATE TABLE pull_requests (
        id          INTEGER PRIMARY KEY,
        repo        TEXT    NOT NULL,
        number      INTEGER NOT NULL,
        updated_at  TEXT    NOT NULL,
        fetched_at  INTEGER NOT NULL,
        data        TEXT    NOT NULL
    );
    CREATE UNIQUE INDEX pull_requests_repo_number ON pull_requests (repo, number);

    -- Ordered membership of pull requests in named lists.
    CREATE TABLE pr_lists (
        list_key  TEXT    NOT NULL,
        pr_id     INTEGER NOT NULL REFERENCES pull_requests (id) ON DELETE CASCADE,
        position  INTEGER NOT NULL,
        PRIMARY KEY (list_key, pr_id)
    );

    -- Changed files, valid only for the pull request's `updated_at` they were fetched at.
    CREATE TABLE pr_files (
        pr_id       INTEGER PRIMARY KEY REFERENCES pull_requests (id) ON DELETE CASCADE,
        updated_at  TEXT    NOT NULL,
        data        TEXT    NOT NULL
    );
"];

/// Applies every migration newer than the database's `user_version`, each in
/// its own transaction, so a failed migration leaves the previous version intact.
pub(crate) fn apply_schema_migrations(connection: &mut Connection) -> Result<()> {
    // Read where this database stands; a fresh file is at version 0.
    let current_version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let applied_count = usize::try_from(current_version).context("negative schema version")?;

    // Apply the rest in order, bumping the version inside the same transaction.
    for (index, sql) in SCHEMA_MIGRATIONS.iter().enumerate().skip(applied_count) {
        let next_version = i64::try_from(index + 1)?;
        let transaction = connection.transaction()?;
        transaction
            .execute_batch(sql)
            .with_context(|| format!("schema migration {next_version}"))?;
        transaction.pragma_update(None, "user_version", next_version)?;
        transaction.commit()?;
    }
    Ok(())
}
