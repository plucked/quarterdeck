use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// The last id handed out, so ids made in the same nanosecond still differ.
static LAST_NOTE_ID: AtomicI64 = AtomicI64::new(0);

/// Identifies a review note. Made by the app when a note is written, so a
/// note can be shown and saved at once without waiting for the database;
/// it is the note's `review_notes.id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NoteId(i64);

impl NoteId {
    /// A new id: the time in nanoseconds, above every id made before in this process.
    pub fn generate() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| {
                i64::try_from(since.as_nanos()).unwrap_or(i64::MAX)
            });
        let previous = LAST_NOTE_ID
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |last| {
                Some(now.max(last + 1))
            })
            .unwrap_or(now);
        Self(now.max(previous + 1))
    }

    /// The id as stored in SQLite.
    pub(crate) fn as_sql_integer(self) -> i64 {
        self.0
    }

    pub(crate) fn from_sql_integer(value: i64) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ids made one after another always grow.
    #[test]
    fn generated_ids_grow() {
        let first = NoteId::generate();
        let second = NoteId::generate();
        assert!(second > first);
    }
}
