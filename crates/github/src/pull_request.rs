use crate::{Label, RepoRef, StackMembership, User};

/// A pull request, normalized from the search API's issue shape, with its
/// place in a stack filled in from GraphQL.
///
/// Timestamps are GitHub's ISO 8601 strings, kept as received; `updated_at`
/// doubles as the version the cache checks derived data against.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Its labels, in GitHub's order.
    pub labels: Vec<Label>,
    /// Where it sits in a stack; `None` for a standalone pull request (and
    /// until the stack membership query answered).
    pub stack: Option<StackMembership>,
}
