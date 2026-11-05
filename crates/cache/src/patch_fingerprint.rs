use qc_github::PullRequestFile;

/// FNV-1a 64-bit parameters: a small hash that stays the same across Rust
/// versions and platforms, unlike `std`'s hasher, so stored values compare.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
/// Separates the hashed fields, so moving text between them changes the hash.
const FIELD_SEPARATOR: u8 = 0;

/// A stable fingerprint of a changed file's patch: equal fingerprints mean
/// the file shows the same change. Stored with a viewed mark to tell whether
/// the file changed since.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatchFingerprint(u64);

impl PatchFingerprint {
    /// Fingerprints the file's name, status and patch.
    pub fn of(file: &PullRequestFile) -> Self {
        let fields = [
            file.filename.as_bytes(),
            file.status.as_bytes(),
            file.patch.as_deref().unwrap_or_default().as_bytes(),
        ];
        let mut hash = FNV_OFFSET_BASIS;
        for field in fields {
            for byte in field.iter().chain([&FIELD_SEPARATOR]) {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
        Self(hash)
    }

    /// The value as stored in SQLite's signed integer column (same bits).
    pub(crate) fn as_sql_integer(self) -> i64 {
        self.0.cast_signed()
    }

    /// Reads a stored value back (same bits).
    pub(crate) fn from_sql_integer(value: i64) -> Self {
        Self(value.cast_unsigned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(patch: &str) -> PullRequestFile {
        PullRequestFile {
            filename: "a.rs".into(),
            previous_filename: None,
            status: "modified".into(),
            additions: 1,
            deletions: 1,
            patch: Some(patch.into()),
        }
    }

    /// The same patch always gives the same fingerprint, a different one another.
    #[test]
    fn fingerprint_follows_patch() {
        assert_eq!(
            PatchFingerprint::of(&file("+a")),
            PatchFingerprint::of(&file("+a"))
        );
        assert_ne!(
            PatchFingerprint::of(&file("+a")),
            PatchFingerprint::of(&file("+b"))
        );
    }

    /// The stored integer reads back as the same fingerprint, high bit included.
    #[test]
    fn sql_integer_round_trips() {
        let fingerprint = PatchFingerprint(u64::MAX - 7);
        assert_eq!(
            PatchFingerprint::from_sql_integer(fingerprint.as_sql_integer()),
            fingerprint
        );
    }
}
