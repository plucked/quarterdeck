use anyhow::Result;
use qc_cache::GitHubCache;
use qc_github::GitHubClient;

use crate::shared_cache::SharedCache;

/// The non-UI subsystems a view needs. Cheap to clone; safe to move into
/// background tasks.
#[derive(Clone)]
pub struct AppServices {
    /// `Err` holds a user-facing reason (e.g. no token); the app then runs on
    /// cached data only.
    pub github: Result<GitHubClient, String>,
    pub cache: SharedCache,
}

impl AppServices {
    /// Opens the default cache and builds a GitHub client from the environment.
    /// Only a cache failure is fatal.
    pub fn initialize() -> Result<Self> {
        let cache = SharedCache::new(GitHubCache::open_default()?);
        let github = GitHubClient::from_env().map_err(|error| error.to_string());
        Ok(Self { github, cache })
    }
}
