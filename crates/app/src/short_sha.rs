//! `short_sha`: a commit SHA as people read it.

/// Characters of a commit SHA shown, as GitHub abbreviates them.
const SHORT_SHA_LENGTH: usize = 7;

/// The first seven characters of a commit SHA (all of a shorter one).
pub fn short_sha(sha: &str) -> &str {
    sha.get(..SHORT_SHA_LENGTH).unwrap_or(sha)
}
