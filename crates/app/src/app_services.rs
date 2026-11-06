use anyhow::Result;
use gpui::Global;
use qd_cache::GitHubCache;
use qd_github::{GitHubClient, TokenSource, resolve_token};

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
}

/// Also a gpui global, for windows that are not handed the services (the
/// settings window clears the cache and shows API usage).
impl Global for AppServices {}

impl AppServices {
    /// Opens the default cache and builds a GitHub client from the token
    /// found in the environment, the Keychain or the `gh` CLI. Only a cache
    /// failure is fatal.
    pub fn initialize() -> Result<Self> {
        let cache = SharedCache::new(GitHubCache::open_default()?);
        let (github, token_source) = match resolve_token() {
            Ok(resolved) => (Ok(GitHubClient::new(resolved.token)), Some(resolved.source)),
            Err(error) => (Err(error.to_string()), None),
        };
        Ok(Self {
            github,
            token_source,
            cache,
        })
    }
}
