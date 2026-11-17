use sha2::{Digest, Sha256};
use std::fmt::Write as _;

use super::BucketCandidate;

/// Length-prefixed typed fields keep operation and receipt digests unambiguous.
pub(super) struct CanonicalDigest(Sha256);

impl CanonicalDigest {
    pub(super) fn new(domain: &str) -> Self {
        let mut digest = Self(Sha256::new());
        digest.text(domain);
        digest
    }

    pub(super) fn text(&mut self, text: &str) {
        self.number(u64::try_from(text.len()).unwrap_or(u64::MAX));
        self.0.update(text.as_bytes());
    }

    pub(super) fn number(&mut self, number: u64) {
        self.0.update(number.to_le_bytes());
    }

    /// Field order and sequence lengths are part of the canonical typed payload.
    pub(super) fn candidate(&mut self, candidate: &BucketCandidate) {
        self.text(&candidate.label);
        self.text(&candidate.description);
        self.0.update([u8::from(candidate.suggest_viewed)]);
        self.number(u64::try_from(candidate.items.len()).unwrap_or(u64::MAX));
        for item in &candidate.items {
            self.text(&item.label);
            self.text(&item.description);
            self.number(u64::try_from(item.paths.len()).unwrap_or(u64::MAX));
            for path in &item.paths {
                self.text(path);
            }
        }
    }

    pub(super) fn finish(self) -> String {
        let mut hexadecimal = String::with_capacity(64);
        for byte in self.0.finalize() {
            let _ = write!(hexadecimal, "{byte:02x}");
        }
        hexadecimal
    }
}
