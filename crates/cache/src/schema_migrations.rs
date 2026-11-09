use anyhow::{Context as _, Result, bail};
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
    r"
    -- How the review pane is laid out and filtered. One row (id = 1).
    CREATE TABLE review_layout (
        id                 INTEGER PRIMARY KEY CHECK (id = 1),
        file_tree_visible  INTEGER NOT NULL CHECK (file_tree_visible IN (0, 1)),
        hide_viewed        INTEGER NOT NULL CHECK (hide_viewed IN (0, 1)),
        hide_tests         INTEGER NOT NULL CHECK (hide_tests IN (0, 1)),
        hide_generated     INTEGER NOT NULL CHECK (hide_generated IN (0, 1))
    );
",
    r"
    -- Where a cached pull request sits in its stack (from GraphQL); all NULL
    -- for a standalone one.
    ALTER TABLE pull_requests ADD COLUMN stack_number INTEGER CHECK (stack_number > 0);
    ALTER TABLE pull_requests ADD COLUMN stack_position INTEGER CHECK (stack_position > 0);
    ALTER TABLE pull_requests ADD COLUMN stack_size INTEGER CHECK (stack_size > 0);
    ALTER TABLE pull_requests ADD COLUMN stack_base_ref TEXT;

    -- A stack with its layers, bottom first, and the tag of the response.
    CREATE TABLE pr_stacks (
        repo_owner    TEXT    NOT NULL,
        repo_name     TEXT    NOT NULL,
        number        INTEGER NOT NULL CHECK (number > 0),
        base_ref      TEXT    NOT NULL,
        is_open       INTEGER NOT NULL CHECK (is_open IN (0, 1)),
        etag          TEXT,
        last_used_at  INTEGER NOT NULL,
        PRIMARY KEY (repo_owner, repo_name, number)
    );

    CREATE TABLE pr_stack_layers (
        repo_owner    TEXT    NOT NULL,
        repo_name     TEXT    NOT NULL,
        stack_number  INTEGER NOT NULL,
        position      INTEGER NOT NULL CHECK (position >= 0),
        -- The pulls API id, not the search id of pull_requests.id.
        pr_id         INTEGER NOT NULL,
        pr_number     INTEGER NOT NULL CHECK (pr_number > 0),
        title         TEXT    NOT NULL,
        is_open       INTEGER NOT NULL CHECK (is_open IN (0, 1)),
        is_draft      INTEGER NOT NULL CHECK (is_draft IN (0, 1)),
        is_merged     INTEGER NOT NULL CHECK (is_merged IN (0, 1)),
        html_url      TEXT    NOT NULL,
        head_ref      TEXT    NOT NULL,
        head_sha      TEXT    NOT NULL,
        base_ref      TEXT    NOT NULL,
        base_sha      TEXT    NOT NULL,
        author_login  TEXT    NOT NULL,
        PRIMARY KEY (repo_owner, repo_name, stack_number, position),
        FOREIGN KEY (repo_owner, repo_name, stack_number)
            REFERENCES pr_stacks (repo_owner, repo_name, number) ON DELETE CASCADE
    );

    -- The changed files between two commits (a combined stack diff). Commits
    -- never change, so a set never goes stale; last_used_at drives cleanup.
    CREATE TABLE compare_file_sets (
        repo_owner    TEXT    NOT NULL,
        repo_name     TEXT    NOT NULL,
        base_sha      TEXT    NOT NULL,
        head_sha      TEXT    NOT NULL,
        last_used_at  INTEGER NOT NULL,
        PRIMARY KEY (repo_owner, repo_name, base_sha, head_sha)
    );

    CREATE TABLE compare_files (
        repo_owner         TEXT    NOT NULL,
        repo_name          TEXT    NOT NULL,
        base_sha           TEXT    NOT NULL,
        head_sha           TEXT    NOT NULL,
        position           INTEGER NOT NULL,
        filename           TEXT    NOT NULL,
        previous_filename  TEXT,
        status             TEXT    NOT NULL,
        additions          INTEGER NOT NULL CHECK (additions >= 0),
        deletions          INTEGER NOT NULL CHECK (deletions >= 0),
        patch              TEXT,
        PRIMARY KEY (repo_owner, repo_name, base_sha, head_sha, position),
        FOREIGN KEY (repo_owner, repo_name, base_sha, head_sha)
            REFERENCES compare_file_sets (repo_owner, repo_name, base_sha, head_sha) ON DELETE CASCADE
    );

    -- Viewed marks of a combined stack review, per range of layers (by the
    -- numbers of its first and last pull request). Like viewed_files, the
    -- fingerprint of a different patch means the file changed.
    CREATE TABLE combined_viewed_files (
        repo_owner         TEXT    NOT NULL,
        repo_name          TEXT    NOT NULL,
        first_number       INTEGER NOT NULL CHECK (first_number > 0),
        last_number        INTEGER NOT NULL CHECK (last_number > 0),
        filename           TEXT    NOT NULL,
        patch_fingerprint  INTEGER NOT NULL,
        PRIMARY KEY (repo_owner, repo_name, first_number, last_number, filename)
    );

    -- The queue's tab, and how stacks are reviewed.
    ALTER TABLE review_queue_view ADD COLUMN queue_tab TEXT NOT NULL DEFAULT 'authored'
        CHECK (queue_tab IN ('authored', 'review_requested'));
    ALTER TABLE review_layout ADD COLUMN stack_review TEXT NOT NULL DEFAULT 'layer'
        CHECK (stack_review IN ('layer', 'combined'));
",
    r"
    -- Whether files moved or copied without changes are hidden.
    ALTER TABLE review_layout ADD COLUMN hide_unchanged_moves INTEGER NOT NULL DEFAULT 0
        CHECK (hide_unchanged_moves IN (0, 1));
",
    r"
    -- The kind filter can hide code too (an option-click on Code). The
    -- other hide_* columns keep their kinds: tests, generated, moved.
    ALTER TABLE review_layout ADD COLUMN hide_code INTEGER NOT NULL DEFAULT 0
        CHECK (hide_code IN (0, 1));
",
    r"
    -- The widths the user dragged the queue sidebar and the review pane's
    -- side panel to, in pixels. One row (id = 1).
    CREATE TABLE panel_widths (
        id                INTEGER PRIMARY KEY CHECK (id = 1),
        sidebar_width     REAL    NOT NULL CHECK (sidebar_width > 0),
        side_panel_width  REAL    NOT NULL CHECK (side_panel_width > 0)
    );
",
    r"
    -- The user's review comments: local notes (for an agent) and drafts of
    -- GitHub comments, kept until submitted. A note belongs to a pull
    -- request (pr_id) or to a combined range of stack layers (repo and the
    -- first and last layer's numbers). It sits on a line (side, line, and
    -- start_line for several lines) or replies to a GitHub thread
    -- (reply_to_thread, its node id). No foreign keys: the cached pull
    -- request may be gone; the user's notes stay.
    CREATE TABLE review_notes (
        id               INTEGER PRIMARY KEY,
        destination      TEXT    NOT NULL CHECK (destination IN ('local', 'github')),
        pr_id            INTEGER,
        repo_owner       TEXT,
        repo_name        TEXT,
        first_number     INTEGER,
        last_number      INTEGER,
        path             TEXT    NOT NULL,
        side             TEXT    CHECK (side IN ('old', 'new')),
        line             INTEGER CHECK (line > 0),
        start_line       INTEGER CHECK (start_line > 0),
        reply_to_thread  TEXT,
        body             TEXT    NOT NULL,
        created_at       INTEGER NOT NULL,
        updated_at       INTEGER NOT NULL,
        CHECK ((pr_id IS NULL) <> (repo_owner IS NULL)),
        CHECK ((line IS NULL) = (reply_to_thread IS NOT NULL)),
        CHECK ((line IS NULL) = (side IS NULL))
    );
    CREATE INDEX review_notes_pull_request ON review_notes (pr_id);

    -- The verdict and summary of a GitHub review being written, per pull request.
    CREATE TABLE review_summaries (
        pr_id  INTEGER PRIMARY KEY,
        event  TEXT    NOT NULL CHECK (event IN ('comment', 'approve', 'request_changes')),
        body   TEXT    NOT NULL
    );
",
    r"
    -- Where new review comments go: GitHub drafts or local notes.
    ALTER TABLE review_layout ADD COLUMN comment_destination TEXT NOT NULL DEFAULT 'github'
        CHECK (comment_destination IN ('local', 'github'));
",
    r"
    -- Whether the queue sidebar shows (1) or is hidden (0).
    ALTER TABLE review_queue_view ADD COLUMN sidebar_visible INTEGER NOT NULL DEFAULT 1
        CHECK (sidebar_visible IN (0, 1));
",
    r"
    -- The sidebar groups the user folded: repositories, and stacks by number.
    CREATE TABLE folded_repositories (
        repo_owner  TEXT NOT NULL,
        repo_name   TEXT NOT NULL,
        PRIMARY KEY (repo_owner, repo_name)
    );
    CREATE TABLE folded_stacks (
        repo_owner    TEXT    NOT NULL,
        repo_name     TEXT    NOT NULL,
        stack_number  INTEGER NOT NULL CHECK (stack_number > 0),
        PRIMARY KEY (repo_owner, repo_name, stack_number)
    );
",
    r"
    -- The color theme picked in Settings → Appearance (one row).
    CREATE TABLE appearance_settings (
        id           INTEGER PRIMARY KEY CHECK (id = 1),
        color_theme  TEXT    NOT NULL CHECK (color_theme IN (
            'ledger', 'vellum', 'overcast', 'signal', 'graphite', 'tide', 'lantern', 'beacon'
        ))
    );
",
    r"
    -- Each changed file's blob SHA after the change (full from the files
    -- API, abbreviated from a whole diff); NULL for rows cached before.
    ALTER TABLE pr_files ADD COLUMN sha TEXT;
    ALTER TABLE compare_files ADD COLUMN sha TEXT;
    -- Whole file contents by blob SHA, for the lines around a diff's hunks.
    -- Content addressed: a row never goes stale, it only expires unused.
    CREATE TABLE file_texts (
        blob_sha      TEXT    PRIMARY KEY CHECK (length(blob_sha) = 40),
        text          TEXT    NOT NULL,
        last_used_at  INTEGER NOT NULL
    );
",
    r"
    -- Changed files cached without blob SHAs keep none while their commits
    -- stay, so their texts could never be found in file_texts. Drop them
    -- (their files go too, ON DELETE CASCADE); the next view fetches them
    -- with SHAs. Viewed marks and notes are kept.
    DELETE FROM pr_file_sets;
    DELETE FROM compare_file_sets;
",
    r"
    -- The labels of cached pull requests, in GitHub's order.
    CREATE TABLE pr_labels (
        pr_id     INTEGER NOT NULL REFERENCES pull_requests (id) ON DELETE CASCADE,
        position  INTEGER NOT NULL CHECK (position >= 0),
        name      TEXT    NOT NULL,
        color     TEXT    NOT NULL,
        PRIMARY KEY (pr_id, position)
    );

    -- The repositories whose other open pull requests the Others tab lists (a preference).
    CREATE TABLE watched_repositories (
        repo_owner  TEXT    NOT NULL,
        repo_name   TEXT    NOT NULL,
        position    INTEGER NOT NULL CHECK (position >= 0),
        PRIMARY KEY (repo_owner, repo_name)
    );

    -- The Others tab's filter of one repository (UI state).
    CREATE TABLE others_filters (
        repo_owner            TEXT    NOT NULL,
        repo_name             TEXT    NOT NULL,
        hide_drafts           INTEGER NOT NULL CHECK (hide_drafts IN (0, 1)),
        hide_review_requests  INTEGER NOT NULL CHECK (hide_review_requests IN (0, 1)),
        updated_within        TEXT    NOT NULL CHECK (updated_within IN ('any', 'week', 'month')),
        PRIMARY KEY (repo_owner, repo_name)
    );
    CREATE TABLE others_filter_chips (
        repo_owner  TEXT NOT NULL,
        repo_name   TEXT NOT NULL,
        facet       TEXT NOT NULL CHECK (facet IN ('label', 'author')),
        value       TEXT NOT NULL,
        choice      TEXT NOT NULL CHECK (choice IN ('only', 'hidden')),
        PRIMARY KEY (repo_owner, repo_name, facet, value),
        FOREIGN KEY (repo_owner, repo_name)
            REFERENCES others_filters (repo_owner, repo_name) ON DELETE CASCADE
    );

    -- The queue's tab gains Others: the column is re-made with the wider CHECK, its value kept.
    CREATE TEMP TABLE saved_queue_tab AS SELECT id, queue_tab FROM review_queue_view;
    ALTER TABLE review_queue_view DROP COLUMN queue_tab;
    ALTER TABLE review_queue_view ADD COLUMN queue_tab TEXT NOT NULL DEFAULT 'authored'
        CHECK (queue_tab IN ('authored', 'review_requested', 'others'));
    UPDATE review_queue_view SET queue_tab =
        (SELECT queue_tab FROM saved_queue_tab WHERE saved_queue_tab.id = review_queue_view.id);
    DROP TABLE saved_queue_tab;
",
    r"
    -- AI-written briefs: user data (paid for), never expired, kept when the
    -- cache is cleared. The latest brief per review scope: a pull request
    -- (pr_id), or a combined range of stack layers (repo + layer numbers).
    CREATE TABLE briefs (
        id              INTEGER PRIMARY KEY,
        pr_id           INTEGER,
        repo_owner      TEXT,
        repo_name       TEXT,
        first_number    INTEGER,
        last_number     INTEGER,
        repository      TEXT    NOT NULL,
        number          INTEGER NOT NULL,
        url             TEXT    NOT NULL,
        subject         TEXT    NOT NULL,
        head_sha        TEXT,
        reader          TEXT    NOT NULL CHECK (reader IN ('expert', 'overview', 'newcomer')),
        code_access     TEXT    NOT NULL CHECK (code_access IN ('workspace', 'diff_only')),
        diff_only_reason TEXT,
        prompt          TEXT    NOT NULL,
        lead            TEXT    NOT NULL,
        cost_usd        REAL,
        session_id      TEXT,
        model           TEXT,
        duration_ms     INTEGER NOT NULL CHECK (duration_ms >= 0),
        written_at      INTEGER NOT NULL,
        CHECK ((pr_id IS NOT NULL AND repo_owner IS NULL AND repo_name IS NULL
                    AND first_number IS NULL AND last_number IS NULL)
            OR (pr_id IS NULL AND repo_owner IS NOT NULL AND repo_name IS NOT NULL
                    AND first_number IS NOT NULL AND last_number IS NOT NULL)),
        CHECK ((code_access = 'diff_only') = (diff_only_reason IS NOT NULL))
    );
    CREATE UNIQUE INDEX briefs_by_pull_request ON briefs (pr_id) WHERE pr_id IS NOT NULL;
    CREATE UNIQUE INDEX briefs_by_range ON briefs (repo_owner, repo_name, first_number, last_number)
        WHERE pr_id IS NULL;

    CREATE TABLE brief_layers (
        brief_id  INTEGER NOT NULL REFERENCES briefs (id) ON DELETE CASCADE,
        position  INTEGER NOT NULL CHECK (position >= 0),
        number    INTEGER NOT NULL,
        title     TEXT    NOT NULL,
        PRIMARY KEY (brief_id, position)
    );
    CREATE TABLE brief_pages (
        brief_id  INTEGER NOT NULL REFERENCES briefs (id) ON DELETE CASCADE,
        position  INTEGER NOT NULL CHECK (position >= 0),
        title     TEXT    NOT NULL,
        summary   TEXT    NOT NULL,
        PRIMARY KEY (brief_id, position)
    );
    -- One row per widget, numbered across the brief; the fields every kind
    -- shares here, the rest in the tables below.
    CREATE TABLE brief_widgets (
        brief_id       INTEGER NOT NULL REFERENCES briefs (id) ON DELETE CASCADE,
        position       INTEGER NOT NULL CHECK (position >= 0),
        page_position  INTEGER NOT NULL CHECK (page_position >= 0),
        kind           TEXT    NOT NULL CHECK (kind IN ('section', 'callout', 'code_excerpt',
                           'code_compare', 'table', 'trace', 'scope', 'tour', 'graph', 'sequence')),
        title          TEXT,
        body           TEXT,
        tone           TEXT CHECK (tone IS NULL OR tone IN ('note', 'constraint', 'verdict', 'evidence')),
        PRIMARY KEY (brief_id, position)
    );
    CREATE TABLE brief_anchors (
        brief_id         INTEGER NOT NULL,
        widget_position  INTEGER NOT NULL,
        slot             TEXT    NOT NULL CHECK (slot IN ('excerpt', 'before', 'after')),
        path             TEXT    NOT NULL,
        side             TEXT    NOT NULL CHECK (side IN ('base', 'head')),
        first_line       INTEGER NOT NULL CHECK (first_line > 0),
        last_line        INTEGER NOT NULL CHECK (last_line >= first_line),
        PRIMARY KEY (brief_id, widget_position, slot),
        FOREIGN KEY (brief_id, widget_position)
            REFERENCES brief_widgets (brief_id, position) ON DELETE CASCADE
    );
    CREATE TABLE brief_table_columns (
        brief_id         INTEGER NOT NULL,
        widget_position  INTEGER NOT NULL,
        position         INTEGER NOT NULL CHECK (position >= 0),
        name             TEXT    NOT NULL,
        PRIMARY KEY (brief_id, widget_position, position),
        FOREIGN KEY (brief_id, widget_position)
            REFERENCES brief_widgets (brief_id, position) ON DELETE CASCADE
    );
    CREATE TABLE brief_table_cells (
        brief_id         INTEGER NOT NULL,
        widget_position  INTEGER NOT NULL,
        row_position     INTEGER NOT NULL CHECK (row_position >= 0),
        column_position  INTEGER NOT NULL CHECK (column_position >= 0),
        text             TEXT    NOT NULL,
        PRIMARY KEY (brief_id, widget_position, row_position, column_position),
        FOREIGN KEY (brief_id, widget_position)
            REFERENCES brief_widgets (brief_id, position) ON DELETE CASCADE
    );
    CREATE TABLE brief_scope_items (
        brief_id         INTEGER NOT NULL,
        widget_position  INTEGER NOT NULL,
        list             TEXT    NOT NULL CHECK (list IN ('changes', 'unchanged', 'mechanical')),
        position         INTEGER NOT NULL CHECK (position >= 0),
        text             TEXT    NOT NULL,
        PRIMARY KEY (brief_id, widget_position, list, position),
        FOREIGN KEY (brief_id, widget_position)
            REFERENCES brief_widgets (brief_id, position) ON DELETE CASCADE
    );
    -- Anchors of steps, boxes and messages: path, side, first and last line; all NULL for none.
    CREATE TABLE brief_tour_steps (
        brief_id         INTEGER NOT NULL,
        widget_position  INTEGER NOT NULL,
        position         INTEGER NOT NULL CHECK (position >= 0),
        title            TEXT    NOT NULL,
        explanation      TEXT    NOT NULL,
        weight           TEXT    NOT NULL CHECK (weight IN ('core', 'supporting', 'mechanical')),
        anchor_path      TEXT,
        anchor_side      TEXT CHECK (anchor_side IS NULL OR anchor_side IN ('base', 'head')),
        anchor_first     INTEGER,
        anchor_last      INTEGER,
        PRIMARY KEY (brief_id, widget_position, position),
        FOREIGN KEY (brief_id, widget_position)
            REFERENCES brief_widgets (brief_id, position) ON DELETE CASCADE
    );
    CREATE TABLE brief_graph_groups (
        brief_id         INTEGER NOT NULL,
        widget_position  INTEGER NOT NULL,
        position         INTEGER NOT NULL CHECK (position >= 0),
        label            TEXT    NOT NULL,
        PRIMARY KEY (brief_id, widget_position, position),
        FOREIGN KEY (brief_id, widget_position)
            REFERENCES brief_widgets (brief_id, position) ON DELETE CASCADE
    );
    CREATE TABLE brief_graph_nodes (
        brief_id         INTEGER NOT NULL,
        widget_position  INTEGER NOT NULL,
        position         INTEGER NOT NULL CHECK (position >= 0),
        label            TEXT    NOT NULL,
        detail           TEXT    NOT NULL,
        role             TEXT    NOT NULL CHECK (role IN ('actor', 'owner', 'transport', 'store',
                             'constraint', 'external')),
        group_position   INTEGER,
        change           TEXT    NOT NULL CHECK (change IN ('same', 'added', 'removed')),
        anchor_path      TEXT,
        anchor_side      TEXT CHECK (anchor_side IS NULL OR anchor_side IN ('base', 'head')),
        anchor_first     INTEGER,
        anchor_last      INTEGER,
        PRIMARY KEY (brief_id, widget_position, position),
        FOREIGN KEY (brief_id, widget_position)
            REFERENCES brief_widgets (brief_id, position) ON DELETE CASCADE
    );
    CREATE TABLE brief_graph_edges (
        brief_id         INTEGER NOT NULL,
        widget_position  INTEGER NOT NULL,
        position         INTEGER NOT NULL CHECK (position >= 0),
        from_position    INTEGER NOT NULL,
        to_position      INTEGER NOT NULL,
        label            TEXT    NOT NULL,
        style            TEXT    NOT NULL CHECK (style IN ('control', 'data', 'async', 'optional')),
        change           TEXT    NOT NULL CHECK (change IN ('same', 'added', 'removed')),
        PRIMARY KEY (brief_id, widget_position, position),
        FOREIGN KEY (brief_id, widget_position)
            REFERENCES brief_widgets (brief_id, position) ON DELETE CASCADE
    );
    CREATE TABLE brief_sequence_lanes (
        brief_id         INTEGER NOT NULL,
        widget_position  INTEGER NOT NULL,
        position         INTEGER NOT NULL CHECK (position >= 0),
        label            TEXT    NOT NULL,
        role             TEXT    NOT NULL CHECK (role IN ('actor', 'owner', 'transport', 'store',
                             'constraint', 'external')),
        PRIMARY KEY (brief_id, widget_position, position),
        FOREIGN KEY (brief_id, widget_position)
            REFERENCES brief_widgets (brief_id, position) ON DELETE CASCADE
    );
    CREATE TABLE brief_sequence_messages (
        brief_id         INTEGER NOT NULL,
        widget_position  INTEGER NOT NULL,
        position         INTEGER NOT NULL CHECK (position >= 0),
        from_lane        INTEGER NOT NULL,
        to_lane          INTEGER NOT NULL,
        label            TEXT    NOT NULL,
        style            TEXT    NOT NULL CHECK (style IN ('call', 'reply', 'async')),
        note             TEXT,
        anchor_path      TEXT,
        anchor_side      TEXT CHECK (anchor_side IS NULL OR anchor_side IN ('base', 'head')),
        anchor_first     INTEGER,
        anchor_last      INTEGER,
        PRIMARY KEY (brief_id, widget_position, position),
        FOREIGN KEY (brief_id, widget_position)
            REFERENCES brief_widgets (brief_id, position) ON DELETE CASCADE
    );
    CREATE TABLE brief_rejected (
        brief_id  INTEGER NOT NULL REFERENCES briefs (id) ON DELETE CASCADE,
        position  INTEGER NOT NULL CHECK (position >= 0),
        widget    TEXT    NOT NULL,
        reason    TEXT    NOT NULL,
        PRIMARY KEY (brief_id, position)
    );
    CREATE TABLE brief_tool_calls (
        brief_id  INTEGER NOT NULL REFERENCES briefs (id) ON DELETE CASCADE,
        position  INTEGER NOT NULL CHECK (position >= 0),
        tool      TEXT    NOT NULL,
        input     TEXT    NOT NULL,
        PRIMARY KEY (brief_id, position)
    );
",
    r"
    -- The branches a pull request comes from and goes into (NULL until GraphQL answered).
    ALTER TABLE pull_requests ADD COLUMN head_ref TEXT;
    ALTER TABLE pull_requests ADD COLUMN base_ref TEXT;

    -- Rules hiding pull requests from every queue tab by branch name: a preference.
    CREATE TABLE hidden_branch_rules (
        position  INTEGER PRIMARY KEY CHECK (position >= 0),
        pattern   TEXT    NOT NULL CHECK (pattern <> ''),
        side      TEXT    NOT NULL CHECK (side IN ('head', 'base', 'either'))
    );
",
];

/// Applies every migration newer than the database's `user_version`, each in
/// its own transaction, so a failed migration leaves the previous version
/// intact. A database a newer app migrated past this one's schema is
/// refused, not opened: this app's queries may no longer fit it, and the
/// newer app can still use it.
pub(crate) fn apply_schema_migrations(connection: &mut Connection) -> Result<()> {
    // Read where this database stands; a fresh file is at version 0.
    let current_version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let applied_count = usize::try_from(current_version).context("negative schema version")?;
    if applied_count > SCHEMA_MIGRATIONS.len() {
        bail!(
            "the database was written by a newer version of Quarterdeck (schema {applied_count}, this version knows {}); install the newest version",
            SCHEMA_MIGRATIONS.len()
        );
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Widening the queue tab's CHECK for Others keeps the saved tab.
    #[test]
    fn keeps_the_saved_tab_when_others_is_added() {
        // Set up: a database at the version before Others, showing the Review tab.
        let mut connection = Connection::open_in_memory().unwrap();
        let before_others = SCHEMA_MIGRATIONS.len() - 1;
        for sql in &SCHEMA_MIGRATIONS[..before_others] {
            connection.execute_batch(sql).unwrap();
        }
        connection
            .pragma_update(None, "user_version", i64::try_from(before_others).unwrap())
            .unwrap();
        connection
            .execute(
                "INSERT INTO review_queue_view (id, detail_scroll_row, detail_scroll_row_offset, queue_tab)
                 VALUES (1, 0, 0, 'review_requested')",
                [],
            )
            .unwrap();

        // Act and expect: the tab survives, and Others is now allowed.
        apply_schema_migrations(&mut connection).unwrap();
        let tab: String = connection
            .query_row("SELECT queue_tab FROM review_queue_view", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(tab, "review_requested");
        connection
            .execute("UPDATE review_queue_view SET queue_tab = 'others'", [])
            .unwrap();
    }

    /// A database migrated by a newer app is refused, and left as it is.
    #[test]
    fn refuses_a_newer_schema() {
        // Set up: a database one migration ahead of this app.
        let mut connection = Connection::open_in_memory().unwrap();
        let newer = i64::try_from(SCHEMA_MIGRATIONS.len() + 1).unwrap();
        connection
            .pragma_update(None, "user_version", newer)
            .unwrap();

        // Act and expect: refused, the version untouched.
        let error = apply_schema_migrations(&mut connection).unwrap_err();
        assert!(error.to_string().contains("newer version of Quarterdeck"));
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, newer);
    }
}
