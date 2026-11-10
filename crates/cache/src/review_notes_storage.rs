use anyhow::{Result, anyhow};
use qd_github::RepoRef;
use rusqlite::{OptionalExtension, Row, params};

use crate::github_enum_sql_text::{
    line_side_as_sql_text, line_side_from_sql_text, review_event_as_sql_text,
    review_event_from_sql_text,
};
use crate::scope_columns::ScopeColumns;
use crate::sql_integers::sql_integer;
use crate::{
    NoteAnchor, NoteDestination, NoteId, ReviewNote, ReviewScope, ReviewSummary, UiStateStore,
};

/// The columns every note query reads, in order.
const NOTE_COLUMNS: &str = "id, destination, pr_id, repo_owner, repo_name, first_number,
    last_number, path, side, line, start_line, reply_to_thread, body, created_at, updated_at";

impl UiStateStore {
    /// The notes of `scope` going to `destination`, by file, then line, then age.
    pub fn review_notes(
        &self,
        scope: &ReviewScope,
        destination: NoteDestination,
    ) -> Result<Vec<ReviewNote>> {
        let columns = ScopeColumns::of(scope)?;
        let mut statement = self.connection().prepare(&format!(
            "SELECT {NOTE_COLUMNS} FROM review_notes
             WHERE pr_id IS ?1 AND repo_owner IS ?2 AND repo_name IS ?3
               AND first_number IS ?4 AND last_number IS ?5 AND destination = ?6
             ORDER BY path, line, created_at, id"
        ))?;
        let rows = statement.query_map(
            columns.as_params_followed_by(params![destination.as_sql_text()]),
            note_from_row,
        )?;
        rows.collect::<rusqlite::Result<Vec<Result<ReviewNote>>>>()?
            .into_iter()
            .collect()
    }

    /// Saves a note, replacing the one with its id (an edit).
    pub fn put_review_note(&self, note: &ReviewNote) -> Result<()> {
        let columns = ScopeColumns::of(&note.scope)?;
        let (side, line, start_line, reply_to_thread) = match &note.anchor {
            NoteAnchor::Line {
                side,
                line,
                start_line,
            } => (
                Some(line_side_as_sql_text(*side)),
                Some(*line),
                *start_line,
                None,
            ),
            NoteAnchor::Reply { thread_id } => (None, None, None, Some(thread_id.as_str())),
        };
        self.connection().execute(
            "INSERT INTO review_notes (pr_id, repo_owner, repo_name, first_number, last_number,
                id, destination, path, side, line, start_line, reply_to_thread, body, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT (id) DO UPDATE SET body = excluded.body, updated_at = excluded.updated_at",
            columns.as_params_followed_by(params![
                note.id.as_sql_integer(),
                note.destination.as_sql_text(),
                note.path,
                side,
                line,
                start_line,
                reply_to_thread,
                note.body,
                note.created_at,
                note.updated_at
            ]),
        )?;
        Ok(())
    }

    /// Removes a note; nothing happens when it is gone already.
    pub fn delete_review_note(&self, id: NoteId) -> Result<()> {
        self.connection().execute(
            "DELETE FROM review_notes WHERE id = ?1",
            [id.as_sql_integer()],
        )?;
        Ok(())
    }

    /// The verdict and summary being written for pull request `pull_request_id`.
    pub fn review_summary(&self, pull_request_id: u64) -> Result<Option<ReviewSummary>> {
        let stored: Option<(String, String)> = self
            .connection()
            .query_row(
                "SELECT event, body FROM review_summaries WHERE pr_id = ?1",
                [sql_integer(pull_request_id)?],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((event, body)) = stored else {
            return Ok(None);
        };
        let event = review_event_from_sql_text(&event)
            .ok_or_else(|| anyhow!("unknown review event {event:?}"))?;
        Ok(Some(ReviewSummary { event, body }))
    }

    /// Saves the verdict and summary being written, replacing the previous ones.
    pub fn put_review_summary(&self, pull_request_id: u64, summary: &ReviewSummary) -> Result<()> {
        self.connection().execute(
            "INSERT INTO review_summaries (pr_id, event, body) VALUES (?1, ?2, ?3)
             ON CONFLICT (pr_id) DO UPDATE SET event = excluded.event, body = excluded.body",
            params![
                sql_integer(pull_request_id)?,
                review_event_as_sql_text(summary.event),
                summary.body
            ],
        )?;
        Ok(())
    }

    /// Forgets the verdict and summary (after the review is submitted).
    pub fn delete_review_summary(&self, pull_request_id: u64) -> Result<()> {
        self.connection().execute(
            "DELETE FROM review_summaries WHERE pr_id = ?1",
            [sql_integer(pull_request_id)?],
        )?;
        Ok(())
    }
}

/// Reads a note from a row of [`NOTE_COLUMNS`]; a value the CHECKs forbid
/// is a damaged row and an error.
fn note_from_row(row: &Row<'_>) -> rusqlite::Result<Result<ReviewNote>> {
    let destination: String = row.get(1)?;
    let pr_id: Option<i64> = row.get(2)?;
    let repo_owner: Option<String> = row.get(3)?;
    let repo_name: Option<String> = row.get(4)?;
    let first_number: Option<i64> = row.get(5)?;
    let last_number: Option<i64> = row.get(6)?;
    let side: Option<String> = row.get(8)?;
    let line: Option<u32> = row.get(9)?;
    let start_line: Option<u32> = row.get(10)?;
    let reply_to_thread: Option<String> = row.get(11)?;
    let id = NoteId::from_sql_integer(row.get(0)?);
    let path: String = row.get(7)?;
    let body: String = row.get(12)?;
    let created_at: i64 = row.get(13)?;
    let updated_at: i64 = row.get(14)?;
    Ok((|| {
        let destination = NoteDestination::from_sql_text(&destination)
            .ok_or_else(|| anyhow!("unknown note destination {destination:?}"))?;
        let scope = match (pr_id, repo_owner, repo_name, first_number, last_number) {
            (Some(pr_id), None, None, None, None) => {
                ReviewScope::PullRequest(u64::try_from(pr_id)?)
            }
            (None, Some(owner), Some(name), Some(first), Some(last)) => {
                ReviewScope::CombinedLayers {
                    repo: RepoRef { owner, name },
                    first_number: u64::try_from(first)?,
                    last_number: u64::try_from(last)?,
                }
            }
            _ => return Err(anyhow!("note {id:?} has a damaged scope")),
        };
        let anchor = match (side.as_deref(), line, reply_to_thread) {
            (Some(side), Some(line), None) => NoteAnchor::Line {
                side: line_side_from_sql_text(side)
                    .ok_or_else(|| anyhow!("note {id:?} has an unknown side {side:?}"))?,
                line,
                start_line,
            },
            (None, None, Some(thread_id)) => NoteAnchor::Reply { thread_id },
            _ => return Err(anyhow!("note {id:?} has a damaged anchor")),
        };
        Ok(ReviewNote {
            id,
            destination,
            scope,
            path,
            anchor,
            body,
            created_at,
            updated_at,
        })
    })())
}

#[cfg(test)]
mod tests {
    use qd_github::{LineSide, ReviewEvent};

    use super::*;

    fn note(scope: ReviewScope, anchor: NoteAnchor, body: &str) -> ReviewNote {
        ReviewNote {
            id: NoteId::generate(),
            destination: NoteDestination::Local,
            scope,
            path: "src/a.rs".into(),
            anchor,
            body: body.into(),
            created_at: 1,
            updated_at: 1,
        }
    }

    fn line(line: u32) -> NoteAnchor {
        NoteAnchor::Line {
            side: LineSide::New,
            line,
            start_line: None,
        }
    }

    /// Notes read back per scope and destination, in line order; an edit replaces the body.
    #[test]
    fn notes_round_trip_per_scope() {
        let store = UiStateStore::open_in_memory().unwrap();
        let pull_request = ReviewScope::PullRequest(7);
        let combined = ReviewScope::CombinedLayers {
            repo: RepoRef {
                owner: "a".into(),
                name: "b".into(),
            },
            first_number: 1,
            last_number: 3,
        };
        let later = note(pull_request.clone(), line(20), "second");
        let mut earlier = note(pull_request.clone(), line(5), "first");
        store.put_review_note(&later).unwrap();
        store.put_review_note(&earlier).unwrap();
        store
            .put_review_note(&note(combined.clone(), line(1), "other"))
            .unwrap();

        earlier.body = "first, edited".into();
        store.put_review_note(&earlier).unwrap();
        let notes = store
            .review_notes(&pull_request, NoteDestination::Local)
            .unwrap();
        assert_eq!(notes, [earlier.clone(), later]);
        assert_eq!(
            store
                .review_notes(&combined, NoteDestination::Local)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            store
                .review_notes(&pull_request, NoteDestination::GitHub)
                .unwrap(),
            []
        );

        store.delete_review_note(earlier.id).unwrap();
        assert_eq!(
            store
                .review_notes(&pull_request, NoteDestination::Local)
                .unwrap()
                .len(),
            1
        );
    }

    /// A reply keeps its thread id.
    #[test]
    fn reply_notes_keep_their_thread() {
        let store = UiStateStore::open_in_memory().unwrap();
        let scope = ReviewScope::PullRequest(7);
        let mut reply = note(
            scope.clone(),
            NoteAnchor::Reply {
                thread_id: "T_1".into(),
            },
            "agreed",
        );
        reply.destination = NoteDestination::GitHub;
        store.put_review_note(&reply).unwrap();
        assert_eq!(
            store.review_notes(&scope, NoteDestination::GitHub).unwrap(),
            [reply]
        );
    }

    /// The summary round-trips and is forgotten after a submission.
    #[test]
    fn summary_round_trips() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert_eq!(store.review_summary(7).unwrap(), None);
        let summary = ReviewSummary {
            event: ReviewEvent::RequestChanges,
            body: "Needs tests".into(),
        };
        store.put_review_summary(7, &summary).unwrap();
        assert_eq!(store.review_summary(7).unwrap(), Some(summary));
        store.delete_review_summary(7).unwrap();
        assert_eq!(store.review_summary(7).unwrap(), None);
    }
}
