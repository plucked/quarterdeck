use anyhow::{Context as _, Result};
use rusqlite::Connection;

/// The schema history, append-only: entry `n` (0-based) moves `PRAGMA
/// user_version` from `n` to `n + 1`. Every value has its own typed column,
/// so a model change is a new entry here.
const SCHEMA_MIGRATIONS: &[&str] = &[
    r"
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
",
    r"
    -- UI state restored after a relaunch. Each table holds at most one row (id = 1).
    CREATE TABLE window_placement (
        id      INTEGER PRIMARY KEY CHECK (id = 1),
        mode    TEXT    NOT NULL CHECK (mode IN ('windowed', 'maximized', 'fullscreen')),
        x       REAL    NOT NULL,
        y       REAL    NOT NULL,
        width   REAL    NOT NULL CHECK (width > 0),
        height  REAL    NOT NULL CHECK (height > 0)
    );

    CREATE TABLE review_queue_view (
        id                    INTEGER PRIMARY KEY CHECK (id = 1),
        -- No foreign key: the pull request may have left the cache; the app falls back.
        selected_pr_id        INTEGER,
        detail_scroll_offset  REAL    NOT NULL
    );
",
    r"
    -- Cached GitHub data moves to typed columns. The cache is discarded rather
    -- than converted: the next refresh fetches it again. UI state is kept.
    DROP TABLE pr_files;
    DROP TABLE pr_lists;
    DROP TABLE pull_requests;

    CREATE TABLE pull_requests (
        id                 INTEGER PRIMARY KEY,
        repo_owner         TEXT    NOT NULL,
        repo_name          TEXT    NOT NULL,
        number             INTEGER NOT NULL CHECK (number > 0),
        title              TEXT    NOT NULL,
        body               TEXT,
        author_login       TEXT    NOT NULL,
        author_avatar_url  TEXT    NOT NULL,
        html_url           TEXT    NOT NULL,
        draft              INTEGER NOT NULL CHECK (draft IN (0, 1)),
        created_at         TEXT    NOT NULL,
        updated_at         TEXT    NOT NULL,
        fetched_at         INTEGER NOT NULL
    );
    CREATE UNIQUE INDEX pull_requests_repo_number
        ON pull_requests (repo_owner, repo_name, number);

    -- Ordered membership of pull requests in named lists.
    CREATE TABLE pr_lists (
        list_key  TEXT    NOT NULL,
        pr_id     INTEGER NOT NULL REFERENCES pull_requests (id) ON DELETE CASCADE,
        position  INTEGER NOT NULL,
        PRIMARY KEY (list_key, pr_id)
    );

    -- A pull request's changed files, valid only for the `updated_at` they were fetched at.
    CREATE TABLE pr_file_sets (
        pr_id       INTEGER PRIMARY KEY REFERENCES pull_requests (id) ON DELETE CASCADE,
        updated_at  TEXT    NOT NULL
    );

    CREATE TABLE pr_files (
        pr_id      INTEGER NOT NULL REFERENCES pr_file_sets (pr_id) ON DELETE CASCADE,
        position   INTEGER NOT NULL,
        filename   TEXT    NOT NULL,
        -- GitHub's value as sent (added, removed, modified, renamed, ...); not
        -- checked, so a status GitHub adds later still caches.
        status     TEXT    NOT NULL,
        additions  INTEGER NOT NULL CHECK (additions >= 0),
        deletions  INTEGER NOT NULL CHECK (deletions >= 0),
        patch      TEXT,
        PRIMARY KEY (pr_id, position)
    );
",
    r"
    -- The path before a rename, for files GitHub reports as renamed.
    ALTER TABLE pr_files ADD COLUMN previous_filename TEXT;

    -- How file diffs are shown. One row (id = 1).
    CREATE TABLE diff_view_settings (
        id          INTEGER PRIMARY KEY CHECK (id = 1),
        layout      TEXT    NOT NULL CHECK (layout IN ('unified', 'split')),
        highlight   TEXT    NOT NULL CHECK (highlight IN ('lines', 'words', 'characters')),
        whitespace  TEXT    NOT NULL CHECK (whitespace IN ('show', 'ignore')),
        wrapping    TEXT    NOT NULL CHECK (wrapping IN ('wrap', 'scroll'))
    );

    -- Files the user marked as viewed, per pull request. The fingerprint is
    -- of the file's patch when marked: a different patch means the file
    -- changed and is no longer viewed. No foreign key: the cached pull
    -- request may be refetched or dropped; the mark is the user's.
    CREATE TABLE viewed_files (
        pr_id              INTEGER NOT NULL,
        filename           TEXT    NOT NULL,
        patch_fingerprint  INTEGER NOT NULL,
        PRIMARY KEY (pr_id, filename)
    );
",
    r"
    -- The detail pane's scroll is kept as the row at its top plus the offset
    -- into it, which restores without knowing row heights. A pixel offset
    -- cannot be converted, so saved positions start again at the top.
    ALTER TABLE review_queue_view
        ADD COLUMN detail_scroll_row INTEGER NOT NULL DEFAULT 0 CHECK (detail_scroll_row >= 0);
    ALTER TABLE review_queue_view
        ADD COLUMN detail_scroll_row_offset REAL NOT NULL DEFAULT 0;
    ALTER TABLE review_queue_view DROP COLUMN detail_scroll_offset;
",
    r"
    -- The display the window was on, so it reopens there; its bounds are
    -- relative to that display. NULL for placements saved before this.
    ALTER TABLE window_placement ADD COLUMN display_uuid TEXT;
",
    r"
    -- File sets record the commits they were fetched for and the tag of the
    -- pull request response that reported them, so a changed updated_at (a
    -- comment) is checked with one free conditional request instead of a
    -- refetch; last_used_at drives cleanup. Existing sets lack the commits
    -- and may be cut off at the first 100 files, so they are refetched.
    DELETE FROM pr_file_sets;
    ALTER TABLE pr_file_sets ADD COLUMN head_sha TEXT NOT NULL DEFAULT '';
    ALTER TABLE pr_file_sets ADD COLUMN base_sha TEXT NOT NULL DEFAULT '';
    ALTER TABLE pr_file_sets ADD COLUMN revision_etag TEXT;
    ALTER TABLE pr_file_sets ADD COLUMN last_used_at INTEGER NOT NULL DEFAULT 0;

    -- How long unused cached GitHub data is kept. One row (id = 1).
    CREATE TABLE cache_settings (
        id         INTEGER PRIMARY KEY CHECK (id = 1),
        ttl_hours  INTEGER NOT NULL CHECK (ttl_hours IN (6, 24, 72, 168))
    );
",
];

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
