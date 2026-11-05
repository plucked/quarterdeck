use std::collections::BTreeSet;

use anyhow::{Result, anyhow};
use qc_file_tree::FileKind;
use rusqlite::{OptionalExtension, params};

use crate::ui_state_store::SINGLE_ROW_ID;
use crate::{NoteDestination, PanelVisibility, ReviewLayoutState, StackReviewMode, UiStateStore};

/// The columns of the `review_layout` row, read before decoding.
struct StoredReviewLayout {
    file_tree_visible: i64,
    stack_review: String,
    comment_destination: String,
    hide_viewed: bool,
    hidden_kinds: BTreeSet<FileKind>,
}

/// The `review_layout` column of each file kind's hide toggle.
fn kind_column(kind: FileKind) -> &'static str {
    match kind {
        FileKind::Code => "hide_code",
        FileKind::Tests => "hide_tests",
        FileKind::Generated => "hide_generated",
        FileKind::Moved => "hide_unchanged_moves",
    }
}

/// Columns read before the kind columns.
const FIXED_COLUMNS: usize = 4;

impl UiStateStore {
    /// The last saved review layout.
    pub fn review_layout(&self) -> Result<Option<ReviewLayoutState>> {
        let kind_columns: Vec<&str> = FileKind::ALL
            .iter()
            .map(|kind| kind_column(*kind))
            .collect();
        let stored = self
            .connection()
            .query_row(
                &format!(
                    "SELECT file_tree_visible, stack_review, hide_viewed, comment_destination, {}
                     FROM review_layout WHERE id = ?1",
                    kind_columns.join(", ")
                ),
                [SINGLE_ROW_ID],
                |row| {
                    let mut hidden_kinds = BTreeSet::new();
                    for (index, kind) in FileKind::ALL.iter().enumerate() {
                        if row.get::<_, bool>(FIXED_COLUMNS + index)? {
                            hidden_kinds.insert(*kind);
                        }
                    }
                    Ok(StoredReviewLayout {
                        file_tree_visible: row.get(0)?,
                        stack_review: row.get(1)?,
                        hide_viewed: row.get(2)?,
                        comment_destination: row.get(3)?,
                        hidden_kinds,
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
        let comment_destination = NoteDestination::from_sql_text(&stored.comment_destination)
            .ok_or_else(|| {
                anyhow!(
                    "unknown comment destination {:?}",
                    stored.comment_destination
                )
            })?;
        Ok(Some(ReviewLayoutState {
            comment_destination,
            file_tree,
            hide_viewed: stored.hide_viewed,
            hidden_kinds: stored.hidden_kinds,
            stack_review,
        }))
    }

    /// Saves the review layout, replacing the previous one.
    pub fn put_review_layout(&self, layout: &ReviewLayoutState) -> Result<()> {
        let hides = |kind| layout.hidden_kinds.contains(&kind);
        self.connection().execute(
            "INSERT INTO review_layout (id, file_tree_visible, stack_review, hide_viewed,
                hide_code, hide_tests, hide_generated, hide_unchanged_moves, comment_destination)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT (id) DO UPDATE SET
                file_tree_visible = excluded.file_tree_visible,
                stack_review = excluded.stack_review,
                hide_viewed = excluded.hide_viewed,
                hide_code = excluded.hide_code, hide_tests = excluded.hide_tests,
                hide_generated = excluded.hide_generated,
                hide_unchanged_moves = excluded.hide_unchanged_moves,
                comment_destination = excluded.comment_destination",
            params![
                SINGLE_ROW_ID,
                layout.file_tree.as_sql_integer(),
                layout.stack_review.as_sql_text(),
                layout.hide_viewed,
                hides(FileKind::Code),
                hides(FileKind::Tests),
                hides(FileKind::Generated),
                hides(FileKind::Moved),
                layout.comment_destination.as_sql_text()
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
            hidden_kinds: BTreeSet::from([FileKind::Code, FileKind::Moved]),
            stack_review: StackReviewMode::Combined,
            comment_destination: NoteDestination::Local,
        };
        store.put_review_layout(&layout).unwrap();
        assert_eq!(store.review_layout().unwrap(), Some(layout));
    }
}
