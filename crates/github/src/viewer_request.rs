use crate::github_client::API_BASE;
use crate::{GitHubClient, GitHubError, User};

impl GitHubClient {
    /// The account the token belongs to: one request, which also tells
    /// whether the token works at all.
    pub fn viewer(&self) -> Result<User, GitHubError> {
        self.get_json(&format!("{API_BASE}/user"), &[])
    }

    /// The login the token belongs to: one request the first time, then
    /// remembered by this client and its clones.
    pub fn viewer_login(&self) -> Result<String, GitHubError> {
        if let Some(login) = self.known_viewer_login().get() {
            return Ok(login.clone());
        }
        let login = self.viewer()?.login;
        Ok(self.known_viewer_login().get_or_init(|| login).clone())
    }
}
