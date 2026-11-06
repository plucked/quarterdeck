/// Where the app's GitHub token came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSource {
    /// The `GITHUB_TOKEN` environment variable (a launch from a terminal).
    Environment,
    /// The token saved in the login Keychain from the app's settings.
    Keychain,
    /// The token the `gh` CLI is logged in with.
    GitHubCli,
}

impl TokenSource {
    /// Where the token came from, completing "Signed in with …".
    pub fn describe(self) -> &'static str {
        match self {
            Self::Environment => "the GITHUB_TOKEN environment variable",
            Self::Keychain => "the token saved in your Keychain",
            Self::GitHubCli => "the GitHub CLI (gh)",
        }
    }
}
