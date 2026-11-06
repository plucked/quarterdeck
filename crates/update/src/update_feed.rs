use ureq::Agent;

use crate::{ReleaseManifest, UpdateError};

/// The manifest of the newest release: GitHub redirects
/// `releases/latest/download/…` to that release's asset, a plain download
/// that needs no token and counts against no API rate limit.
const LATEST_MANIFEST_URL: &str =
    "https://github.com/plucked/quarterdeck/releases/latest/download/latest.json";
const USER_AGENT: &str = concat!("quarterdeck/", env!("CARGO_PKG_VERSION"));

/// Where releases are announced: the newest GitHub release's `latest.json`.
/// Cheap to clone; blocking, so the app calls it on a background executor.
#[derive(Clone)]
pub struct UpdateFeed {
    agent: Agent,
}

impl Default for UpdateFeed {
    fn default() -> Self {
        Self {
            agent: Agent::config_builder()
                .http_status_as_error(false)
                .user_agent(USER_AGENT)
                .build()
                .into(),
        }
    }
}

impl UpdateFeed {
    /// The newest release's manifest (one small download).
    pub fn latest_release(&self) -> Result<ReleaseManifest, UpdateError> {
        let mut response = self.agent.get(LATEST_MANIFEST_URL).call()?;
        let status = response.status();
        if !status.is_success() {
            return Err(UpdateError::Status {
                status: status.as_u16(),
            });
        }
        let json = response.body_mut().read_to_string()?;
        ReleaseManifest::parse(&json)
    }

    /// The HTTP agent, for the download of the release itself.
    pub(crate) fn agent(&self) -> &Agent {
        &self.agent
    }
}
