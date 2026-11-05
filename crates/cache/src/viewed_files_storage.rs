use anyhow::{Context as _, Result};
use rusqlite::params;

use crate::{PatchFingerprint, UiStateStore, ViewedFile};

impl UiStateStore {
    /// The files of a pull request marked as viewed, with the fingerprints
    /// they were marked at.
    pub fn viewed_files(&self, pull_request_id: u64) -> Result<Vec<ViewedFile>> {
        let mut statement = self.connection().prepare(
            "SELECT filename, patch_fingerprint FROM viewed_files WHERE pr_id = ?1 ORDER BY filename",
        )?;
        let rows = statement.query_map([sql_id(pull_request_id)?], |row| {
            Ok(ViewedFile {
                filename: row.get(0)?,
                fingerprint: PatchFingerprint::from_sql_integer(row.get(1)?),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Marks a file as viewed at its current fingerprint, replacing an older mark.
    pub fn mark_file_viewed(&self, pull_request_id: u64, file: &ViewedFile) -> Result<()> {
        self.connection().execute(
            "INSERT INTO viewed_files (pr_id, filename, patch_fingerprint) VALUES (?1, ?2, ?3)
             ON CONFLICT (pr_id, filename) DO UPDATE SET patch_fingerprint = excluded.patch_fingerprint",
            params![
                sql_id(pull_request_id)?,
                file.filename,
                file.fingerprint.as_sql_integer()
            ],
        )?;
        Ok(())
    }

    /// Removes a file's viewed mark; nothing happens when it has none.
    pub fn unmark_file_viewed(&self, pull_request_id: u64, filename: &str) -> Result<()> {
        self.connection().execute(
            "DELETE FROM viewed_files WHERE pr_id = ?1 AND filename = ?2",
            params![sql_id(pull_request_id)?, filename],
        )?;
        Ok(())
    }
}

/// A GitHub id as SQLite's signed integer, failing instead of wrapping.
fn sql_id(id: u64) -> Result<i64> {
    i64::try_from(id).context("pull request id exceeds SQLite's integer range")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewed(filename: &str, fingerprint: i64) -> ViewedFile {
        ViewedFile {
            filename: filename.into(),
            fingerprint: PatchFingerprint::from_sql_integer(fingerprint),
        }
    }

    /// Marks are per pull request, a new mark replaces the old one, and unmarking removes it.
    #[test]
    fn viewed_marks_round_trip() {
        let store = UiStateStore::open_in_memory().unwrap();
        store.mark_file_viewed(1, &viewed("a.rs", 10)).unwrap();
        store.mark_file_viewed(1, &viewed("a.rs", 11)).unwrap();
        store.mark_file_viewed(1, &viewed("b.rs", 20)).unwrap();
        store.mark_file_viewed(2, &viewed("a.rs", 30)).unwrap();
        assert_eq!(
            store.viewed_files(1).unwrap(),
            [viewed("a.rs", 11), viewed("b.rs", 20)]
        );

        store.unmark_file_viewed(1, "a.rs").unwrap();
        assert_eq!(store.viewed_files(1).unwrap(), [viewed("b.rs", 20)]);
    }
}
