use std::process::Command;

use crate::GitHubError;

/// Environment variable checked first for a personal access token.
const TOKEN_ENV_VAR: &str = "GITHUB_TOKEN";

/// Resolves a GitHub token: `GITHUB_TOKEN` first, then the token the `gh` CLI
/// is logged in with.
pub fn resolve_token() -> Result<String, GitHubError> {
    // An explicit environment token wins, so a different account can be used per launch.
    if let Ok(token) = std::env::var(TOKEN_ENV_VAR)
        && !token.trim().is_empty()
    {
        return Ok(token.trim().to_owned());
    }

    // Fall back to the `gh` CLI; a missing binary counts as no token.
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .map_err(|_| GitHubError::NoToken)?;
    let token = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if output.status.success() && !token.is_empty() {
        Ok(token)
    } else {
        Err(GitHubError::NoToken)
    }
}
