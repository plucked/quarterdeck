use crate::PatchFingerprint;

/// A file the user marked as viewed, with the fingerprint of the patch they saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewedFile {
    pub filename: String,
    pub fingerprint: PatchFingerprint,
}
