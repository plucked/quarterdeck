use crate::github_client::API_BASE;
use crate::{GitHubClient, GitHubError, User};

impl GitHubClient {
    /// The account the token belongs to: one request, which also tells
    /// whether the token works at all.
    pub fn viewer(&self) -> Result<User, GitHubError> {
        self.get_json(&format!("{API_BASE}/user"), &[])
    }
}
