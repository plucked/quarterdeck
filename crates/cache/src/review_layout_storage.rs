use std::collections::BTreeSet;

use anyhow::{Result, anyhow};
use qc_file_tree::FileGroup;
use rusqlite::{OptionalExtension, params};

use crate::ui_state_store::SINGLE_ROW_ID;
use crate::{PanelVisibility, ReviewLayoutState, StackReviewMode, UiStateStore};

/// The columns of the `review_layout` row, read before decoding.
struct StoredReviewLayout {
    file_tree_visible: i64,
    hidden_groups: BTreeSet<FileGroup>,
    stack_review: String,
}

/// The `review_layout` column of each file group's hide toggle.
fn group_column(group: FileGroup) -> &'static str {
    match group {
        FileGroup::Viewed => "hide_viewed",
        FileGroup::Tests => "hide_tests",
        FileGroup::Generated => "hide_generated",
        FileGroup::UnchangedMoves => "hide_unchanged_moves",
    }
}

impl UiStateStore {
    /// The last saved review layout.
    pub fn review_layout(&self) -> Result<Option<ReviewLayoutState>> {
        let group_columns: Vec<&str> = FileGroup::ALL
            .iter()
            .map(|group| group_column(*group))
            .collect();
        let stored = self
            .connection()
            .query_row(
                &format!(
                    "SELECT file_tree_visible, stack_review, {} FROM review_layout WHERE id = ?1",
                    group_columns.join(", ")
                ),
                [SINGLE_ROW_ID],
                |row| {
                    let mut hidden_groups = BTreeSet::new();
                    for (index, group) in FileGroup::ALL.iter().enumerate() {
                        if row.get::<_, bool>(index + 2)? {
                            hidden_groups.insert(*group);
                        }
                    }
                    Ok(StoredReviewLayout {
                        file_tree_visible: row.get(0)?,
                        stack_review: row.get(1)?,
                        hidden_groups,
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
        let stack_review = StackReviewMode::from_sql_text(&stored.stack_review)
            .ok_or_else(|| anyhow!("unknown stack review mode {:?}", stored.stack_review))?;
        Ok(Some(ReviewLayoutState {
            file_tree,
            hidden_groups: stored.hidden_groups,
            stack_review,
        }))
    }

    /// Saves the review layout, replacing the previous one.
    pub fn put_review_layout(&self, layout: &ReviewLayoutState) -> Result<()> {
        let hides = |group| layout.hidden_groups.contains(&group);
        self.connection().execute(
            "INSERT INTO review_layout (id, file_tree_visible, stack_review,
                hide_viewed, hide_tests, hide_generated, hide_unchanged_moves)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (id) DO UPDATE SET
                file_tree_visible = excluded.file_tree_visible,
                stack_review = excluded.stack_review,
                hide_viewed = excluded.hide_viewed, hide_tests = excluded.hide_tests,
                hide_generated = excluded.hide_generated,
                hide_unchanged_moves = excluded.hide_unchanged_moves",
            params![
                SINGLE_ROW_ID,
                layout.file_tree.as_sql_integer(),
                layout.stack_review.as_sql_text(),
                hides(FileGroup::Viewed),
                hides(FileGroup::Tests),
                hides(FileGroup::Generated),
                hides(FileGroup::UnchangedMoves)
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
            hidden_groups: BTreeSet::from([FileGroup::Viewed, FileGroup::UnchangedMoves]),
            stack_review: StackReviewMode::Combined,
        };
        store.put_review_layout(&layout).unwrap();
        assert_eq!(store.review_layout().unwrap(), Some(layout));
    }
}
