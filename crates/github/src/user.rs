use serde::{Deserialize, Serialize};

/// A GitHub account as embedded in other objects (author, reviewer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub avatar_url: String,
}
