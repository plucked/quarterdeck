use qc_cache::{NoteId, ReviewNote, ReviewSummary};

/// A change to the user's review notes, waiting to be written.
pub enum ReviewChange {
    /// A new note, or an edited one (same id).
    PutNote(ReviewNote),
    DeleteNote(NoteId),
    /// The verdict and summary of pull request `.0`'s GitHub review.
    PutSummary(u64, ReviewSummary),
    /// Forgets them (the review was submitted).
    DeleteSummary(u64),
}
