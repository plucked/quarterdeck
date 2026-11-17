use anyhow::Result;
use gpui::Global;
use qd_cache::GitHubCache;
use qd_github::{GitHubClient, TokenSource, resolve_token};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::shared_cache::SharedCache;

/// The non-UI subsystems a view needs. Cheap to clone; safe to move into
/// background tasks.
#[derive(Clone)]
pub struct AppServices {
    /// `Err` holds a user-facing reason (e.g. no token); the app then runs on
    /// cached data only.
    pub github: Result<GitHubClient, String>,
    /// Where the client's token came from; `None` without a client.
    pub token_source: Option<TokenSource>,
    pub cache: SharedCache,
    pub database: PathBuf,
}

/// Also a gpui global, for windows that are not handed the services (the
/// settings window clears the cache and shows API usage).
impl Global for AppServices {}

impl AppServices {
    /// Opens the account profile and builds a GitHub client from the token
    /// found in the environment, the Keychain or the `gh` CLI. Only a cache
    /// failure is fatal.
    pub fn initialize() -> Result<Self> {
        let profiles = qd_cache::accounts_directory()?;
        let anonymous = profiles.join("signed-out.sqlite");
        let (github, token_source, database) = match resolve_token() {
            Ok(resolved) => {
                let binding = credential_binding(&profiles, &resolved.token);
                let client = GitHubClient::new(resolved.token);
                match resolve_profile(&profiles, &binding, || client.viewer_id())? {
                    Ok(database) => (Ok(client), Some(resolved.source), database),
                    Err(error) => (Err(error.to_string()), Some(resolved.source), anonymous),
                }
            }
            Err(error) => (Err(error.to_string()), None, anonymous),
        };
        let cache = SharedCache::new(GitHubCache::open(&database)?);
        Ok(Self {
            github,
            token_source,
            cache,
            database,
        })
    }
}

fn resolve_profile(
    root: &Path,
    binding: &Path,
    identify: impl FnOnce() -> Result<u64, qd_github::GitHubError>,
) -> Result<Result<PathBuf, qd_github::GitHubError>> {
    use std::io::Write as _;
    if let Ok(value) = std::fs::read_to_string(binding)
        && let Ok(id) = value.parse::<u64>()
    {
        return Ok(Ok(account_database(root, id)));
    }
    let id = match identify() {
        Ok(id) => id,
        Err(error) => return Ok(Err(error)),
    };
    let directory = binding.parent().expect("credential bindings have a parent");
    std::fs::create_dir_all(directory)?;
    let mut temp = tempfile::NamedTempFile::new_in(directory)?;
    write!(temp, "{id}")?;
    temp.persist(binding)?;
    Ok(Ok(account_database(root, id)))
}

fn account_database(profiles: &Path, id: u64) -> PathBuf {
    profiles.join(format!("{id}.sqlite"))
}

fn credential_binding(profiles: &Path, token: &str) -> PathBuf {
    use std::fmt::Write as _;
    let mut hash = String::with_capacity(64);
    for byte in Sha256::digest(token.as_bytes()) {
        let _ = write!(hash, "{byte:02x}");
    }
    profiles.join("credentials").join(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accounts_and_offline_bindings_are_isolated() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let profile = |token, id| {
            resolve_profile(root, &credential_binding(root, token), || Ok(id))
                .unwrap()
                .unwrap()
        };
        let first = profile("first", 42);
        let second = profile("second", 43);
        assert_ne!(first, second);
        assert_eq!(first, profile("rotated", 42));
        let offline = resolve_profile(root, &credential_binding(root, "first"), || {
            panic!("known credential should work offline")
        })
        .unwrap()
        .unwrap();
        assert_eq!(first, offline);
        assert!(
            resolve_profile(root, &credential_binding(root, "unknown"), || Err(
                qd_github::GitHubError::NoToken
            ))
            .unwrap()
            .is_err()
        );
        assert!(
            !credential_binding(root, "first")
                .to_string_lossy()
                .contains("first")
        );
    }
}
