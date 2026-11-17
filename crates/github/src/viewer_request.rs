use crate::github_client::API_BASE;
use crate::{GitHubClient, GitHubError, User};

impl GitHubClient {
    /// The account the token belongs to: one request, which also tells
    /// whether the token works at all.
    pub fn viewer(&self) -> Result<User, GitHubError> {
        self.get_json(&format!("{API_BASE}/user"), &[])
    }

    /// Stable account identity across login and credential changes.
    pub fn viewer_id(&self) -> Result<u64, GitHubError> {
        #[derive(serde::Deserialize)]
        struct Account {
            id: u64,
            login: String,
        }
        // First sign-in runs before the main window opens. Bound that lookup.
        let request = self
            .with_headers(
                self.agent().get(format!("{API_BASE}/user")),
                crate::github_client::JSON_MEDIA_TYPE,
                None,
            )
            .config()
            .timeout_global(Some(std::time::Duration::from_secs(10)))
            .build();
        let response = self.read_response(request.call()?, &mut |_| {})?;
        let account: Account = serde_json::from_str(&response.body)?;
        self.known_viewer_login().get_or_init(|| account.login);
        Ok(account.id)
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
