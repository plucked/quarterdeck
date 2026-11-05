use anyhow::{Result, anyhow};
use rusqlite::{OptionalExtension, params};

use crate::ui_state_store::SINGLE_ROW_ID;
use crate::{PanelVisibility, ReviewLayoutState, UiStateStore};

/// The columns of the `review_layout` row, read before decoding.
struct StoredReviewLayout {
    file_tree_visible: i64,
    hide_viewed: bool,
    hide_tests: bool,
    hide_generated: bool,
}

impl UiStateStore {
    /// The last saved review layout.
    pub fn review_layout(&self) -> Result<Option<ReviewLayoutState>> {
        let stored = self
            .connection()
            .query_row(
                "SELECT file_tree_visible, hide_viewed, hide_tests, hide_generated
                 FROM review_layout WHERE id = ?1",
                [SINGLE_ROW_ID],
                |row| {
                    Ok(StoredReviewLayout {
                        file_tree_visible: row.get(0)?,
                        hide_viewed: row.get(1)?,
                        hide_tests: row.get(2)?,
                        hide_generated: row.get(3)?,
                    })
                },
            )
            .optional()?;
        let Some(stored) = stored else {
            return Ok(None);
        };
        // The CHECK constraints make other values impossible short of a hand edit.
        let file_tree = PanelVisibility::from_sql_integer(stored.file_tree_visible)
            .ok_or_else(|| anyhow!("unknown file tree visibility {}", stored.file_tree_visible))?;
        Ok(Some(ReviewLayoutState {
            file_tree,
            hide_viewed: stored.hide_viewed,
            hide_tests: stored.hide_tests,
            hide_generated: stored.hide_generated,
        }))
    }

    /// Saves the review layout, replacing the previous one.
    pub fn put_review_layout(&self, layout: &ReviewLayoutState) -> Result<()> {
        self.connection().execute(
            "INSERT INTO review_layout (id, file_tree_visible, hide_viewed, hide_tests, hide_generated)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (id) DO UPDATE SET
                file_tree_visible = excluded.file_tree_visible, hide_viewed = excluded.hide_viewed,
                hide_tests = excluded.hide_tests, hide_generated = excluded.hide_generated",
            params![
                SINGLE_ROW_ID,
                layout.file_tree.as_sql_integer(),
                layout.hide_viewed,
                layout.hide_tests,
                layout.hide_generated
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A saved layout reads back, and a fresh database has none.
    #[test]
    fn review_layout_round_trips() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert_eq!(store.review_layout().unwrap(), None);
        let layout = ReviewLayoutState {
            file_tree: PanelVisibility::Hidden,
            hide_viewed: true,
            hide_tests: false,
            hide_generated: true,
        };
        store.put_review_layout(&layout).unwrap();
        assert_eq!(store.review_layout().unwrap(), Some(layout));
    }
}
