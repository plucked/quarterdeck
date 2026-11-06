use std::process::Command;

use crate::keychain_token::stored_token;
use crate::{GitHubError, ResolvedToken, TokenSource};

/// Environment variable checked first for a personal access token.
const TOKEN_ENV_VAR: &str = "GITHUB_TOKEN";
/// Where the `gh` CLI is looked for: on `PATH`, then where Homebrew puts it
/// (Apple silicon, then Intel). An app started from Finder gets launchd's
/// `PATH`, which has neither Homebrew folder.
const GH_CLI_PATHS: [&str; 3] = ["gh", "/opt/homebrew/bin/gh", "/usr/local/bin/gh"];

/// Resolves a GitHub token: `GITHUB_TOKEN` first, then the token saved in
/// the Keychain from the app's settings, then the one the `gh` CLI is
/// logged in with.
pub fn resolve_token() -> Result<ResolvedToken, GitHubError> {
    // An explicit environment token wins, so a different account can be used per launch.
    if let Ok(token) = std::env::var(TOKEN_ENV_VAR)
        && !token.trim().is_empty()
    {
        return Ok(ResolvedToken {
            token: token.trim().to_owned(),
            source: TokenSource::Environment,
        });
    }

    // A token the user saved in the app; a Keychain that cannot be read counts as none.
    match stored_token() {
        Ok(Some(token)) => {
            return Ok(ResolvedToken {
                token,
                source: TokenSource::Keychain,
            });
        }
        Ok(None) => {}
        Err(error) => eprintln!("cannot read the saved GitHub token: {error}"),
    }

    // Fall back to the `gh` CLI wherever it is installed.
    GH_CLI_PATHS
        .iter()
        .find_map(|program| gh_cli_token(program))
        .map(|token| ResolvedToken {
            token,
            source: TokenSource::GitHubCli,
        })
        .ok_or(GitHubError::NoToken)
}

/// The token `program auth token` prints; `None` when the program is
/// missing or not logged in.
fn gh_cli_token(program: &str) -> Option<String> {
    let output = Command::new(program)
        .args(["auth", "token"])
        .output()
        .ok()?;
    let token = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !token.is_empty()).then_some(token)
}
