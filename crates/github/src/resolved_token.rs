use crate::TokenSource;

/// A GitHub token and where it was found.
#[derive(Debug, Clone)]
pub struct ResolvedToken {
    pub token: String,
    pub source: TokenSource,
}
