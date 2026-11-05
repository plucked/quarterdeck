use anyhow::Result;
use rusqlite::{Connection, params};

use crate::sql_integers::sql_integer;
use crate::{PatchFingerprint, ReviewScope, UiStateStore, ViewedFile};

impl UiStateStore {
    /// The files marked as viewed in `scope`, with the fingerprints they
    /// were marked at, in name order.
    pub fn viewed_files(&self, scope: &ReviewScope) -> Result<Vec<ViewedFile>> {
        match scope {
            ReviewScope::PullRequest(pull_request_id) => read_marks(
                self.connection(),
                "SELECT filename, patch_fingerprint FROM viewed_files WHERE pr_id = ?1
                 ORDER BY filename",
                &[&sql_integer(*pull_request_id)?],
            ),
            ReviewScope::CombinedLayers {
                repo,
                first_number,
                last_number,
            } => read_marks(
                self.connection(),
                "SELECT filename, patch_fingerprint FROM combined_viewed_files
                 WHERE repo_owner = ?1 AND repo_name = ?2 AND first_number = ?3 AND last_number = ?4
                 ORDER BY filename",
                &[
                    &repo.owner,
                    &repo.name,
                    &sql_integer(*first_number)?,
                    &sql_integer(*last_number)?,
                ],
            ),
        }
    }

    /// Marks a file as viewed in `scope` at its current fingerprint,
    /// replacing an older mark.
    pub fn mark_file_viewed(&self, scope: &ReviewScope, file: &ViewedFile) -> Result<()> {
        match scope {
            ReviewScope::PullRequest(pull_request_id) => self.connection().execute(
                "INSERT INTO viewed_files (pr_id, filename, patch_fingerprint) VALUES (?1, ?2, ?3)
                 ON CONFLICT (pr_id, filename) DO UPDATE SET patch_fingerprint = excluded.patch_fingerprint",
                params![
                    sql_integer(*pull_request_id)?,
                    file.filename,
                    file.fingerprint.as_sql_integer()
                ],
            )?,
            ReviewScope::CombinedLayers {
                repo,
                first_number,
                last_number,
            } => self.connection().execute(
                "INSERT INTO combined_viewed_files
                    (repo_owner, repo_name, first_number, last_number, filename, patch_fingerprint)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (repo_owner, repo_name, first_number, last_number, filename)
                 DO UPDATE SET patch_fingerprint = excluded.patch_fingerprint",
                params![
                    repo.owner,
                    repo.name,
                    sql_integer(*first_number)?,
                    sql_integer(*last_number)?,
                    file.filename,
                    file.fingerprint.as_sql_integer()
                ],
            )?,
        };
        Ok(())
    }

    /// Removes a file's viewed mark in `scope`; nothing happens when it has none.
    pub fn unmark_file_viewed(&self, scope: &ReviewScope, filename: &str) -> Result<()> {
        match scope {
            ReviewScope::PullRequest(pull_request_id) => self.connection().execute(
                "DELETE FROM viewed_files WHERE pr_id = ?1 AND filename = ?2",
                params![sql_integer(*pull_request_id)?, filename],
            )?,
            ReviewScope::CombinedLayers {
                repo,
                first_number,
                last_number,
            } => self.connection().execute(
                "DELETE FROM combined_viewed_files
                 WHERE repo_owner = ?1 AND repo_name = ?2 AND first_number = ?3
                   AND last_number = ?4 AND filename = ?5",
                params![
                    repo.owner,
                    repo.name,
                    sql_integer(*first_number)?,
                    sql_integer(*last_number)?,
                    filename
                ],
            )?,
        };
        Ok(())
    }
}

/// Runs a marks query whose columns are `filename, patch_fingerprint`.
fn read_marks(
    connection: &Connection,
    sql: &str,
    parameters: &[&dyn rusqlite::ToSql],
) -> Result<Vec<ViewedFile>> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map(parameters, |row| {
        Ok(ViewedFile {
            filename: row.get(0)?,
            fingerprint: PatchFingerprint::from_sql_integer(row.get(1)?),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use qd_github::RepoRef;

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
        let first = ReviewScope::PullRequest(1);
        store.mark_file_viewed(&first, &viewed("a.rs", 10)).unwrap();
        store.mark_file_viewed(&first, &viewed("a.rs", 11)).unwrap();
        store.mark_file_viewed(&first, &viewed("b.rs", 20)).unwrap();
        store
            .mark_file_viewed(&ReviewScope::PullRequest(2), &viewed("a.rs", 30))
            .unwrap();
        assert_eq!(
            store.viewed_files(&first).unwrap(),
            [viewed("a.rs", 11), viewed("b.rs", 20)]
        );

        store.unmark_file_viewed(&first, "a.rs").unwrap();
        assert_eq!(store.viewed_files(&first).unwrap(), [viewed("b.rs", 20)]);
    }

    /// Combined marks belong to their range of layers alone.
    #[test]
    fn combined_marks_are_per_range() {
        let store = UiStateStore::open_in_memory().unwrap();
        let range = |last_number| ReviewScope::CombinedLayers {
            repo: RepoRef {
                owner: "a".into(),
                name: "b".into(),
            },
            first_number: 1,
            last_number,
        };
        store
            .mark_file_viewed(&range(3), &viewed("a.rs", 1))
            .unwrap();
        assert_eq!(store.viewed_files(&range(3)).unwrap(), [viewed("a.rs", 1)]);
        assert_eq!(store.viewed_files(&range(4)).unwrap(), []);
        store.unmark_file_viewed(&range(3), "a.rs").unwrap();
        assert_eq!(store.viewed_files(&range(3)).unwrap(), []);
    }
}
