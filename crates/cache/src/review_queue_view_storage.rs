use anyhow::{Context as _, Result, anyhow};
use rusqlite::{OptionalExtension, params};

use crate::ui_state_store::SINGLE_ROW_ID;
use crate::{DetailScrollPosition, PanelVisibility, QueueTab, ReviewQueueViewState, UiStateStore};

/// The columns of the `review_queue_view` row, read before decoding.
struct StoredReviewQueueView {
    selected_id: Option<i64>,
    scroll_row: i64,
    scroll_row_offset: f32,
    queue_tab: String,
    sidebar_visible: i64,
}

impl UiStateStore {
    /// The last saved review queue state.
    pub fn review_queue_view_state(&self) -> Result<Option<ReviewQueueViewState>> {
        let row = self
            .connection()
            .query_row(
                "SELECT selected_pr_id, detail_scroll_row, detail_scroll_row_offset, queue_tab,
                        sidebar_visible
                 FROM review_queue_view WHERE id = ?1",
                [SINGLE_ROW_ID],
                |row| {
                    Ok(StoredReviewQueueView {
                        selected_id: row.get(0)?,
                        scroll_row: row.get(1)?,
                        scroll_row_offset: row.get(2)?,
                        queue_tab: row.get(3)?,
                        sidebar_visible: row.get(4)?,
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
        let tab = QueueTab::from_sql_text(&stored.queue_tab)
            .ok_or_else(|| anyhow!("unknown queue tab {:?}", stored.queue_tab))?;
        let sidebar = PanelVisibility::from_sql_integer(stored.sidebar_visible)
            .ok_or_else(|| anyhow!("unknown sidebar visibility {}", stored.sidebar_visible))?;
        Ok(Some(ReviewQueueViewState {
            tab,
            sidebar,
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
        self.connection().execute(
            "INSERT INTO review_queue_view (id, selected_pr_id, detail_scroll_row, detail_scroll_row_offset, queue_tab, sidebar_visible)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (id) DO UPDATE SET
                selected_pr_id = excluded.selected_pr_id,
                detail_scroll_row = excluded.detail_scroll_row,
                detail_scroll_row_offset = excluded.detail_scroll_row_offset,
                queue_tab = excluded.queue_tab,
                sidebar_visible = excluded.sidebar_visible",
            params![
                SINGLE_ROW_ID,
                selected_id,
                i64::try_from(state.detail_scroll.row)?,
                state.detail_scroll.offset_in_row,
                state.tab.as_sql_text(),
                state.sidebar.as_sql_integer()
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh database has no saved review queue state.
    #[test]
    fn fresh_database_has_no_review_queue_state() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert!(store.review_queue_view_state().unwrap().is_none());
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
            tab: QueueTab::ReviewRequested,
            sidebar: PanelVisibility::Hidden,
        };
        store.put_review_queue_view_state(&selected).unwrap();
        assert_eq!(store.review_queue_view_state().unwrap(), Some(selected));

        let cleared = ReviewQueueViewState::default();
        store.put_review_queue_view_state(&cleared).unwrap();
        assert_eq!(store.review_queue_view_state().unwrap(), Some(cleared));
    }
}
