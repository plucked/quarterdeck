use anyhow::Result;
use qd_github::FileText;
use rusqlite::params;

use crate::GitHubCache;
use crate::unix_now::unix_now;

/// Length of a full git blob SHA in hex digits.
const FULL_SHA_LENGTH: usize = 40;
/// The shortest abbreviated SHA looked up: git abbreviates to at least 7.
const MIN_ABBREVIATED_SHA_LENGTH: usize = 7;
/// Sorts after every lowercase hex digit, so `prefix..prefix + this` holds
/// exactly the SHAs starting with `prefix`.
const PAST_HEX_DIGITS: char = 'g';

impl GitHubCache {
    /// The cached content of the blob `blob_sha`, and marks it used now.
    /// `blob_sha` may be abbreviated (as a whole diff writes it): then the
    /// one cached blob starting with it, and none when several do. `None`
    /// when not cached. A blob's content never changes, so a hit is current.
    pub fn file_text(&self, blob_sha: &str) -> Result<Option<FileText>> {
        // Hex digits only, so the prefix range below holds just SHAs.
        let is_hex = blob_sha.bytes().all(|byte| byte.is_ascii_hexdigit());
        let is_usable = (MIN_ABBREVIATED_SHA_LENGTH..=FULL_SHA_LENGTH).contains(&blob_sha.len());
        if !is_hex || !is_usable {
            return Ok(None);
        }

        // Find the blob with a range on the key (an index seek, unlike
        // `LIKE`); two matches for a prefix are ambiguous, so neither counts.
        let prefix = blob_sha.to_ascii_lowercase();
        let past_prefix = format!("{prefix}{PAST_HEX_DIGITS}");
        let mut statement = self.connection().prepare(
            "SELECT blob_sha, text FROM file_texts WHERE blob_sha >= ?1 AND blob_sha < ?2 LIMIT 2",
        )?;
        let mut matches: Vec<FileText> = statement
            .query_map(params![prefix, past_prefix], |row| {
                Ok(FileText {
                    blob_sha: row.get(0)?,
                    text: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let (Some(found), None) = (matches.pop(), matches.pop()) else {
            return Ok(None);
        };

        // Being read is a use: it moves the blob's expiry.
        self.connection().execute(
            "UPDATE file_texts SET last_used_at = ?2 WHERE blob_sha = ?1",
            params![found.blob_sha, unix_now()],
        )?;
        Ok(Some(found))
    }

    /// Stores `file_text` under its blob SHA (lowercase, full length),
    /// replacing the same blob stored before.
    pub fn put_file_text(&self, file_text: &FileText) -> Result<()> {
        self.connection().execute(
            "INSERT INTO file_texts (blob_sha, text, last_used_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (blob_sha) DO UPDATE SET
                text = excluded.text, last_used_at = excluded.last_used_at",
            params![
                file_text.blob_sha.to_ascii_lowercase(),
                file_text.text,
                unix_now()
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(blob_sha: &str, text: &str) -> FileText {
        FileText {
            blob_sha: blob_sha.into(),
            text: text.into(),
        }
    }

    const BLOB: &str = "1234567890abcdef1234567890abcdef12345678";

    /// A stored blob is found by its full SHA and by an abbreviation of it.
    #[test]
    fn finds_blobs_by_full_and_abbreviated_sha() {
        let cache = GitHubCache::open_in_memory().unwrap();
        cache.put_file_text(&text(BLOB, "a\nb\n")).unwrap();
        assert_eq!(cache.file_text(BLOB).unwrap(), Some(text(BLOB, "a\nb\n")));
        assert_eq!(
            cache.file_text("1234567").unwrap(),
            Some(text(BLOB, "a\nb\n"))
        );
        assert_eq!(cache.file_text("7654321").unwrap(), None);
    }

    /// An abbreviation two blobs share, a too short one and a non-hex one find nothing.
    #[test]
    fn ambiguous_or_malformed_shas_find_nothing() {
        let cache = GitHubCache::open_in_memory().unwrap();
        cache.put_file_text(&text(BLOB, "one")).unwrap();
        let other = "1234567ffffffffffffffffffffffffffffffff0";
        cache.put_file_text(&text(other, "two")).unwrap();
        assert_eq!(cache.file_text("1234567").unwrap(), None);
        assert_eq!(cache.file_text("12345").unwrap(), None);
        assert_eq!(cache.file_text("1234567%").unwrap(), None);
        assert_eq!(cache.file_text(other).unwrap(), Some(text(other, "two")));
    }
}
