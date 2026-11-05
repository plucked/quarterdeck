use serde::Deserialize;

/// A GitHub account as embedded in other objects (author, reviewer).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct User {
    pub login: String,
    pub avatar_url: String,
}
