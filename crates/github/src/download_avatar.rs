use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

use ureq::Agent;

use crate::GitHubError;

/// Where GitHub serves avatars; nothing else is downloaded as one.
const AVATAR_HOST: &str = "https://avatars.githubusercontent.com/";
/// The largest avatar accepted: GitHub's are a few KB at small sizes.
const MAX_AVATAR_BYTES: u64 = 1024 * 1024;
/// How long connecting to the avatar host may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// How long one whole avatar download may take, so a stalled connection
/// cannot hold its thread for good.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);

/// The one agent every avatar download shares, so they reuse connections.
fn avatar_agent() -> &'static Agent {
    static AGENT: OnceLock<Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(DOWNLOAD_TIMEOUT))
            .build()
            .into()
    })
}

/// The image (PNG or JPEG bytes) of the avatar at `avatar_url` (a `User`'s
/// `avatar_url`), `size_px` pixels square. Sent without the token: the
/// avatar host is public and is not the API.
pub fn download_avatar(avatar_url: &str, size_px: u32) -> Result<Vec<u8>, GitHubError> {
    if !avatar_url.starts_with(AVATAR_HOST) {
        return Err(GitHubError::Api {
            status: 0,
            body: format!("not a GitHub avatar: {avatar_url}"),
        });
    }

    // GitHub scales the avatar to `s` pixels.
    let separator = if avatar_url.contains('?') { '&' } else { '?' };
    let url = format!("{avatar_url}{separator}s={size_px}");
    let mut response = avatar_agent().get(&url).call()?;
    let status = response.status();
    if !status.is_success() {
        return Err(GitHubError::Api {
            status: status.as_u16(),
            body: String::new(),
        });
    }

    // Read at most the limit, so a wrong answer cannot fill the memory.
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_AVATAR_BYTES)
        .read_to_end(&mut bytes)
        .map_err(ureq::Error::from)?;
    Ok(bytes)
}
