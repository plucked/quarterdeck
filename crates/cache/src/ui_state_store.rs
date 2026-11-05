use std::path::Path;

use anyhow::{Context as _, Result, anyhow};
use rusqlite::{Connection, OptionalExtension, params};

use crate::database_connection::{default_database_path, open_database};
use crate::{DetailScrollPosition, ReviewQueueViewState, WindowMode, WindowPlacement};

/// The single row id of the one-row UI state tables.
pub(crate) const SINGLE_ROW_ID: i64 = 1;

/// The columns of the `review_queue_view` row, read before decoding.
struct StoredReviewQueueView {
    selected_id: Option<i64>,
    scroll_row: i64,
    scroll_row_offset: f32,
}

/// A connection to the database for the UI state the app restores after a
/// relaunch: window placement and review queue state here, diff view
/// settings and viewed files in `diff_view_settings_storage.rs` and
/// `viewed_files_storage.rs`. Separate from [`crate::GitHubCache`]'s connection so UI writes
/// never wait behind a cache write; WAL lets both share the file.
///
/// Reads return `Ok(None)` when nothing was saved yet; every write is one
/// statement.
pub struct UiStateStore {
    connection: Connection,
}

impl UiStateStore {
    /// Opens (creating if needed) the default database file.
    pub fn open_default() -> Result<Self> {
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

    /// The connection, for the storage of other UI state topics in sibling files.
    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }

    /// The last saved window placement.
    pub fn window_placement(&self) -> Result<Option<WindowPlacement>> {
        let row = self
            .connection
            .query_row(
                "SELECT mode, x, y, width, height FROM window_placement WHERE id = ?1",
                [SINGLE_ROW_ID],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        WindowPlacement {
                            mode: WindowMode::Windowed,
                            x: row.get(1)?,
                            y: row.get(2)?,
                            width: row.get(3)?,
                            height: row.get(4)?,
                        },
                    ))
                },
            )
            .optional()?;
        let Some((mode_text, placement)) = row else {
            return Ok(None);
        };

        // The CHECK constraint makes an unknown mode impossible short of a hand edit.
        let mode = WindowMode::from_sql_text(&mode_text)
            .ok_or_else(|| anyhow!("unknown window mode {mode_text:?}"))?;
        Ok(Some(WindowPlacement { mode, ..placement }))
    }

    /// Saves the window placement, replacing the previous one.
    pub fn put_window_placement(&self, placement: &WindowPlacement) -> Result<()> {
        self.connection.execute(
            "INSERT INTO window_placement (id, mode, x, y, width, height)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (id) DO UPDATE SET
                mode = excluded.mode, x = excluded.x, y = excluded.y,
                width = excluded.width, height = excluded.height",
            params![
                SINGLE_ROW_ID,
                placement.mode.as_sql_text(),
                placement.x,
                placement.y,
                placement.width,
                placement.height
            ],
        )?;
        Ok(())
    }

    /// The last saved review queue state.
    pub fn review_queue_view_state(&self) -> Result<Option<ReviewQueueViewState>> {
        let row = self
            .connection
            .query_row(
                "SELECT selected_pr_id, detail_scroll_row, detail_scroll_row_offset
                 FROM review_queue_view WHERE id = ?1",
                [SINGLE_ROW_ID],
                |row| {
                    Ok(StoredReviewQueueView {
                        selected_id: row.get(0)?,
                        scroll_row: row.get(1)?,
                        scroll_row_offset: row.get(2)?,
                    })
                },
            )
            .optional()?;
        let Some(stored) = row else {
            return Ok(None);
        };

        // GitHub ids and row indexes are never negative; a negative value is a damaged row.
        let selected_pull_request_id = stored
            .selected_id
            .map(u64::try_from)
            .transpose()
            .context("negative pull request id")?;
        Ok(Some(ReviewQueueViewState {
            selected_pull_request_id,
            detail_scroll: DetailScrollPosition {
                row: usize::try_from(stored.scroll_row).context("negative scroll row")?,
                offset_in_row: stored.scroll_row_offset,
            },
        }))
    }

    /// Saves the review queue state, replacing the previous one.
    pub fn put_review_queue_view_state(&self, state: &ReviewQueueViewState) -> Result<()> {
        let selected_id = state
            .selected_pull_request_id
            .map(i64::try_from)
            .transpose()
            .context("pull request id exceeds SQLite's integer range")?;
        self.connection.execute(
            "INSERT INTO review_queue_view (id, selected_pr_id, detail_scroll_row, detail_scroll_row_offset)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (id) DO UPDATE SET
                selected_pr_id = excluded.selected_pr_id,
                detail_scroll_row = excluded.detail_scroll_row,
                detail_scroll_row_offset = excluded.detail_scroll_row_offset",
            params![
                SINGLE_ROW_ID,
                selected_id,
                i64::try_from(state.detail_scroll.row)?,
                state.detail_scroll.offset_in_row
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh database has no saved UI state.
    #[test]
    fn fresh_database_has_no_ui_state() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert!(store.window_placement().unwrap().is_none());
        assert!(store.review_queue_view_state().unwrap().is_none());
    }

    /// The latest window placement replaces the previous one.
    #[test]
    fn window_placement_keeps_latest() {
        let store = UiStateStore::open_in_memory().unwrap();
        let first = WindowPlacement {
            mode: WindowMode::Windowed,
            x: 10.,
            y: 20.,
            width: 800.,
            height: 600.,
        };
        let second = WindowPlacement {
            mode: WindowMode::Maximized,
            ..first
        };
        store.put_window_placement(&first).unwrap();
        store.put_window_placement(&second).unwrap();
        assert_eq!(store.window_placement().unwrap(), Some(second));
    }

    /// The review queue state round-trips, including no selection.
    #[test]
    fn review_queue_view_state_round_trips() {
        let store = UiStateStore::open_in_memory().unwrap();
        let selected = ReviewQueueViewState {
            selected_pull_request_id: Some(42),
            detail_scroll: DetailScrollPosition {
                row: 120,
                offset_in_row: 6.5,
            },
        };
        store.put_review_queue_view_state(&selected).unwrap();
        assert_eq!(store.review_queue_view_state().unwrap(), Some(selected));

        let cleared = ReviewQueueViewState::default();
        store.put_review_queue_view_state(&cleared).unwrap();
        assert_eq!(store.review_queue_view_state().unwrap(), Some(cleared));
    }

    /// The schema rejects a zero-size window instead of storing it.
    #[test]
    fn schema_rejects_empty_window() {
        let store = UiStateStore::open_in_memory().unwrap();
        let empty = WindowPlacement {
            mode: WindowMode::Windowed,
            x: 0.,
            y: 0.,
            width: 0.,
            height: 0.,
        };
        assert!(store.put_window_placement(&empty).is_err());
    }
}
