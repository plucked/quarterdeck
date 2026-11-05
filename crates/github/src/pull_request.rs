use serde::{Deserialize, Serialize};

use crate::{RepoRef, User};

/// A pull request, normalized from the search API's issue shape.
///
/// Timestamps are GitHub's ISO 8601 strings, kept as received; `updated_at`
/// doubles as the version the cache checks derived data against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequest {
    /// GitHub's global id, stable across renames and transfers.
    pub id: u64,
    pub number: u64,
    pub repo: RepoRef,
    pub title: String,
    pub body: Option<String>,
    pub author: User,
    pub html_url: String,
    pub draft: bool,
    pub created_at: String,
    pub updated_at: String,
}
