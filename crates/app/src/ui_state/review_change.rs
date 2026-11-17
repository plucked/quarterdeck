use qd_cache::{NoteId, ReviewNote, ReviewSummary};

/// A change to the user's review notes, waiting to be written.
#[derive(Clone)]
pub enum ReviewChange {
    /// A new note, or an edited one (same id).
    PutNote(ReviewNote),
    DeleteNote(NoteId),
    /// The verdict and summary of pull request `.0`'s GitHub review.
    PutSummary(u64, ReviewSummary),
    /// Forgets them (the review was submitted).
    DeleteSummary(u64, ReviewSummary),
}

impl ReviewChange {
    pub(super) fn write(&self, store: &qd_cache::UiStateStore) -> anyhow::Result<()> {
        match self {
            Self::PutNote(note) => store.put_review_note(note),
            Self::DeleteNote(id) => store.delete_review_note(*id),
            Self::PutSummary(id, summary) => store.put_review_summary(*id, summary),
            Self::DeleteSummary(id, submitted) => store.clear_submitted_summary(*id, submitted),
        }
    }

    pub(super) fn overlay(
        &self,
        scope: &qd_cache::ReviewScope,
        notes: &mut Vec<ReviewNote>,
        summary: &mut Option<ReviewSummary>,
    ) {
        match self {
            Self::PutNote(note) if &note.scope == scope => {
                notes.retain(|n| n.id != note.id);
                notes.push(note.clone());
            }
            Self::DeleteNote(id) => notes.retain(|n| n.id != *id),
            Self::PutSummary(id, s) if scope == &qd_cache::ReviewScope::PullRequest(*id) => {
                *summary = Some(s.clone());
            }
            Self::DeleteSummary(id, s)
                if scope == &qd_cache::ReviewScope::PullRequest(*id)
                    && summary.as_ref() == Some(s) =>
            {
                *summary = None;
            }
            _ => {}
        }
    }
}
