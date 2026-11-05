use std::process::Command;

use crate::{Error, Result};

/// Resolve a GitHub token: `GITHUB_TOKEN` env var first, then `gh auth token`.
pub fn resolve_token() -> Result<String> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN")
        && !token.trim().is_empty()
    {
        return Ok(token.trim().to_owned());
    }

    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .map_err(|_| Error::NoToken)?;
    let token = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if output.status.success() && !token.is_empty() {
        Ok(token)
    } else {
        Err(Error::NoToken)
    }
}
